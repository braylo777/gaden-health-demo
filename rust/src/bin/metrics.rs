//! `cargo run --release --bin metrics` -> writes `../metrics.json` and `../data/fit_result.json`.
//! Flags: `--no-tests` skips the `cargo test` count, `--no-cov` skips `cargo llvm-cov`.
//! Every number is measured by this run; nothing is typed in by hand.

use gaden::{controllers, fit, hormones::*, sim};
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn json_f(v: f64) -> String {
    if v.is_finite() {
        format!("{:.4}", v)
    } else {
        "null".into()
    }
}

fn count_tests(root: &Path) -> Option<(usize, usize)> {
    let out = Command::new("cargo")
        .args(["test", "--release", "--", "--test-threads=4"])
        .current_dir(root)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (mut passed, mut failed) = (0, 0);
    for line in text.lines().filter(|l| l.starts_with("test result:")) {
        let num = |key: &str| -> usize {
            line.split(';')
                .find(|s| s.contains(key))
                .and_then(|s| s.split_whitespace().rev().nth(1))
                .and_then(|n| n.trim_end_matches('.').parse().ok())
                .unwrap_or(0)
        };
        passed += num(" passed");
        failed += num(" failed");
    }
    Some((passed + failed, passed))
}

fn coverage(root: &Path) -> Option<f64> {
    let out = Command::new("cargo")
        .args([
            "llvm-cov",
            "--summary-only",
            "--json",
            "--ignore-filename-regex",
            "src/bin/",
        ])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let tail = &s[s.rfind("\"totals\"")?..];
    let lines = &tail[tail.find("\"lines\"")?..];
    let pct = &lines[lines.find("\"percent\":")? + 10..];
    let end = pct.find([',', '}'])?;
    pct[..end].trim().parse().ok()
}

fn loc_rust(dir: &Path) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("._") {
                continue; // macOS metadata files on exFAT
            }
            if p.is_dir() {
                n += loc_rust(&p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                n += std::fs::read_to_string(&p)
                    .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                    .unwrap_or(0);
            }
        }
    }
    n
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let t0 = std::time::Instant::now();

    // 1. fit
    let ts = fit::targets();
    let base = Params::prior();
    let f = fit::fit(&base, &ts, 600);
    let (loo, held) = fit::leave_one_out(&base, &ts, 300);
    eprintln!(
        "fit rmse {:.4}  loo {:.4}  ({:.1}s)",
        f.rmse,
        loo,
        t0.elapsed().as_secs_f64()
    );
    let p = f.params.clone();

    // 2. stress suite, every controller (+ open loop as a reference)
    let c = sim::calibrate_coupling(&p, &Coupling::default(), 30.0); // re-derived each run
    eprintln!("calibrated w_ghr {:.2}", c.w_ghr);
    let suite = sim::suite();
    let mut reps = vec![];
    for name in controllers::NAMES.iter().chain(["open"].iter()) {
        let r = sim::run_suite(name, &suite, &p, &c);
        eprintln!(
            "{:7} pass {:5.1}%  worst AgRP {:.1} Hz",
            name,
            100.0 * r.pass_rate(),
            r.worst_agrp_hz
        );
        reps.push(r);
    }
    let best = reps[..3]
        .iter()
        .max_by(|a, b| {
            a.pass_rate()
                .total_cmp(&b.pass_rate())
                .then(b.worst_agrp_hz.total_cmp(&a.worst_agrp_hz))
        })
        .unwrap();

    // 3. nominal day peak with the best controller
    let mut bc = controllers::by_name(&best.controller, 1).unwrap();
    let nom = sim::run_day(&sim::nominal(), Some(bc.as_mut()), &p, &c);
    let nom_open = sim::run_day(&sim::nominal(), None, &p, &c);

    // 4. tests + coverage
    let tests = if args.iter().any(|a| a == "--no-tests") {
        None
    } else {
        count_tests(root)
    };
    let cov = if args.iter().any(|a| a == "--no-cov") {
        None
    } else {
        coverage(root)
    };
    let loc = loc_rust(&root.join("src")) + loc_rust(&root.join("tests"));

    // per-category pass for the best controller
    let mut cats: Vec<(&str, usize, usize)> = vec![];
    for sc in &suite {
        let ok = !best.failures.contains(&sc.name);
        match cats.iter_mut().find(|x| x.0 == sc.category) {
            Some(x) => {
                x.1 += 1;
                x.2 += ok as usize;
            }
            None => cats.push((sc.category, 1, ok as usize)),
        }
    }

    let mut j = String::from("{\n");
    let _ = writeln!(j, "  \"agrp_peak_hz\": {},", json_f(nom.agrp_max_hz));
    let _ = writeln!(
        j,
        "  \"agrp_peak_open_loop_hz\": {},",
        json_f(nom_open.agrp_max_hz)
    );
    let _ = writeln!(j, "  \"stress_pass\": {},", json_f(best.pass_rate()));
    let _ = writeln!(j, "  \"stress_pass_controller\": \"{}\",", best.controller);
    let _ = writeln!(j, "  \"scenarios\": {},", suite.len());
    j.push_str("  \"controllers\": {\n");
    for (i, r) in reps.iter().enumerate() {
        let _ = writeln!(
            j,
            "    \"{}\": {{\"stress_pass\": {}, \"worst_agrp_hz\": {}, \"mean_frac_agrp_below_2hz\": {}, \"mean_pomc_err_hz\": {}, \"mean_u_agrp_ua\": {}}}{}",
            r.controller,
            json_f(r.pass_rate()),
            json_f(r.worst_agrp_hz),
            json_f(r.mean_frac_below_2hz),
            json_f(r.mean_pomc_err_hz),
            json_f(r.mean_u_agrp),
            if i + 1 < reps.len() { "," } else { "" }
        );
    }
    j.push_str("  },\n  \"stress_by_category\": {");
    let cs: Vec<String> = cats
        .iter()
        .map(|(k, n, ok)| format!("\"{k}\": {{\"n\": {n}, \"pass\": {ok}}}"))
        .collect();
    let _ = writeln!(j, "{}}},", cs.join(", "));
    let (nt, np) = tests
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap_or(("null".into(), "null".into()));
    let _ = writeln!(j, "  \"tests\": {nt},\n  \"tests_passing\": {np},");
    let _ = writeln!(j, "  \"loc_rust\": {loc},");
    let _ = writeln!(
        j,
        "  \"coverage\": {},",
        cov.map(|v| json_f(v / 100.0)).unwrap_or("null".into())
    );
    let _ = writeln!(j, "  \"w_ghr_calibrated\": {},", json_f(c.w_ghr));
    let _ = writeln!(j, "  \"fit_rmse\": {},", json_f(f.rmse));
    let _ = writeln!(j, "  \"fit_loo_rmse\": {},", json_f(loo));
    let _ = writeln!(j, "  \"fit_targets\": {},", ts.len());
    let _ = writeln!(j, "  \"fit_free_params\": {},", fit::FIT_KEYS.len());
    let _ = writeln!(j, "  \"note\": \"Simulation only. Not medical advice. fit_* are relative RMSE vs published summary values (data/SOURCES.md).\"");
    j.push_str("}\n");
    std::fs::write(root.join("../metrics.json"), &j).expect("write metrics.json");

    // fit detail
    let mut fj = String::from("{\n  \"residuals\": [\n");
    let rows: Vec<String> = f
        .residuals
        .iter()
        .map(|(id, m, r)| {
            let t = ts.iter().find(|t| &t.id == id).unwrap();
            let lo = held.iter().find(|h| &h.0 == id).map(|h| h.1).unwrap_or(f64::NAN);
            format!(
                "    {{\"id\": \"{id}\", \"target\": {}, \"model\": {}, \"rel_residual\": {}, \"loo_rel_residual\": {}, \"source\": \"{}\"}}",
                json_f(t.value), json_f(*m), json_f(*r), json_f(lo), t.source
            )
        })
        .collect();
    fj.push_str(&rows.join(",\n"));
    let _ = write!(
        fj,
        "\n  ],\n  \"params\": {{\"lep_mean\": {}, \"lep_a1\": {}, \"lep_p1\": {}, \"lep_a2\": {}, \"ghr_rise\": {}, \"ghr_tau_s\": {}, \"glp1_tau_s\": {}, \"sleep_lep\": {}, \"sleep_ghr\": {}}}\n}}\n",
        json_f(p.lep_mean), json_f(p.lep_a1), json_f(p.lep_p1), json_f(p.lep_a2), json_f(p.ghr_rise),
        json_f(p.ghr_tau_s), json_f(p.glp1_tau_s), json_f(p.sleep_lep), json_f(p.sleep_ghr)
    );
    std::fs::write(root.join("../data/fit_result.json"), &fj).expect("write fit_result.json");
    print!("{j}");
    eprintln!("done in {:.1}s", t0.elapsed().as_secs_f64());
}
