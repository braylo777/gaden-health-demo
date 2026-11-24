//! `cargo run --release --bin metrics` -> writes `../results/metrics.json` (and a copy at
//! `../metrics.json`) and `../data/fit_result.json`.
//! Flags: `--no-tests` skips the `cargo test` count, `--no-cov` skips `cargo llvm-cov`,
//! `--out <path>` writes the metrics file there only (used by CI to check reproduction).
//! Every number is measured by this run; nothing is typed in by hand.
//! File layout: `{generated_at, git_sha, machine, value: {...}}`.

use gaden::{anchors, controllers, fit, hormones::*, sim};
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn jf(v: f64) -> String {
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

fn cmd(prog: &str, args: &[&str], dir: &Path) -> String {
    Command::new(prog)
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// UTC time as ISO 8601 (no external crate): civil-from-days.
fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

struct Row {
    name: &'static str,
    n: usize,
    passed: usize,
    worst_agrp: f64,
    frac_below: f64,
    pomc_err: f64,
    u_agrp: f64,
    by_kind: Vec<(&'static str, usize, usize)>,
}

fn run_row(
    name: &'static str,
    suite: &[sim::Scenario],
    p: &Params,
    c: &Coupling,
    threads: usize,
) -> Row {
    let out = sim::run_suite_par(name, suite, p, c, threads);
    let n = out.len();
    let mut row = Row {
        name,
        n,
        passed: 0,
        worst_agrp: 0.0,
        frac_below: 0.0,
        pomc_err: 0.0,
        u_agrp: 0.0,
        by_kind: vec![],
    };
    for o in &out {
        row.passed += o.passed as usize;
        row.worst_agrp = row.worst_agrp.max(o.result.agrp_max_hz);
        row.frac_below += o.result.frac_agrp_below_2hz;
        row.pomc_err += (o.result.pomc_mean_hz - 8.0).abs();
        row.u_agrp += o.result.u_mean[0];
        match row.by_kind.iter_mut().find(|k| k.0 == o.kind) {
            Some(k) => {
                k.1 += 1;
                k.2 += o.passed as usize;
            }
            None => row.by_kind.push((o.kind, 1, o.passed as usize)),
        }
    }
    let nf = n.max(1) as f64;
    row.frac_below /= nf;
    row.pomc_err /= nf;
    row.u_agrp /= nf;
    row
}

fn row_json(r: &Row) -> String {
    let (lo, hi) = sim::wilson_ci(r.passed, r.n);
    let kinds: Vec<String> = r
        .by_kind
        .iter()
        .map(|(k, n, ok)| {
            let (a, b) = sim::wilson_ci(*ok, *n);
            format!(
                "\"{k}\": {{\"n\": {n}, \"passed\": {ok}, \"pass\": {}, \"ci95\": [{}, {}]}}",
                jf(*ok as f64 / (*n).max(1) as f64),
                jf(a),
                jf(b)
            )
        })
        .collect();
    format!(
        "\"{}\": {{\"n\": {}, \"passed\": {}, \"pass\": {}, \"ci95\": [{}, {}], \"worst_agrp_hz\": {}, \"mean_frac_agrp_below_2hz\": {}, \"mean_pomc_err_hz\": {}, \"mean_u_agrp_ua\": {}, \"by_kind\": {{{}}}}}",
        r.name,
        r.n,
        r.passed,
        jf(r.passed as f64 / r.n.max(1) as f64),
        jf(lo),
        jf(hi),
        jf(r.worst_agrp),
        jf(r.frac_below),
        jf(r.pomc_err),
        jf(r.u_agrp),
        kinds.join(", ")
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = root.join("..");
    let t0 = std::time::Instant::now();
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned();

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
    let c = sim::calibrate_coupling(&p, &Coupling::default(), 30.0);
    eprintln!("calibrated w_ghr {:.2}", c.w_ghr);

    // 2. the 1,000-scenario suite, every controller + open loop
    let suite = sim::suite_1000();
    let mut rows = vec![];
    for name in controllers::ALL_NAMES.iter().chain(["open"].iter()) {
        let t = std::time::Instant::now();
        let r = run_row(name, &suite, &p, &c, threads);
        eprintln!(
            "{:10} pass {:5.1}%  worst AgRP {:5.1} Hz  ({:.1}s)",
            name,
            100.0 * r.passed as f64 / r.n as f64,
            r.worst_agrp,
            t.elapsed().as_secs_f64()
        );
        rows.push(r);
    }
    let rate = |r: &Row| r.passed as f64 / r.n.max(1) as f64;
    let closed: Vec<&Row> = rows.iter().filter(|r| r.name != "open").collect();
    let best = closed
        .iter()
        .max_by(|a, b| {
            rate(a)
                .total_cmp(&rate(b))
                .then(b.worst_agrp.total_cmp(&a.worst_agrp))
        })
        .unwrap();
    let best_safe = closed
        .iter()
        .filter(|r| r.name.starts_with("safe_"))
        .max_by(|a, b| rate(a).total_cmp(&rate(b)))
        .unwrap();

    // 3. the v0.2 240-scenario suite (kept for comparison)
    let legacy = sim::suite();
    let legacy_rows: Vec<Row> = controllers::NAMES
        .iter()
        .map(|n| run_row(n, &legacy, &p, &c, threads))
        .collect();

    // 4. nominal day, timing
    let mut bc = controllers::by_name(best.name, 1).unwrap();
    let tn = std::time::Instant::now();
    let nom = sim::run_day(&sim::nominal(), Some(bc.as_mut()), &p, &c);
    let ms_day = tn.elapsed().as_secs_f64() * 1000.0;
    let nom_open = sim::run_day(&sim::nominal(), None, &p, &c);

    // 5. tests + coverage + code size
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

    // 6. literature anchors
    let a_gastric = anchors::first_hour_emptying_ratio(&p);
    let a_misalign = anchors::misalign_glucose_ratio(&p);
    let a_car = anchors::car_peak_minutes(&p);

    // ---- JSON
    let mut v = String::new();
    let _ = writeln!(v, "    \"scenarios\": {},", suite.len());
    let _ = writeln!(v, "    \"scenario_seed\": {},", sim::STRESS_SEED);
    let kinds: Vec<String> = sim::STRESS_KINDS
        .iter()
        .map(|k| {
            format!(
                "\"{k}\": {}",
                suite.iter().filter(|s| s.category == *k).count()
            )
        })
        .collect();
    let _ = writeln!(v, "    \"scenarios_by_kind\": {{{}}},", kinds.join(", "));
    let _ = writeln!(v, "    \"best_controller\": \"{}\",", best.name);
    let _ = writeln!(v, "    \"best_pass\": {},", jf(rate(best)));
    let _ = writeln!(v, "    \"best_safe_controller\": \"{}\",", best_safe.name);
    let _ = writeln!(v, "    \"best_safe_pass\": {},", jf(rate(best_safe)));
    let _ = writeln!(v, "    \"open_pass\": {},", jf(rate(rows.last().unwrap())));
    let _ = writeln!(v, "    \"controllers\": {{");
    for (i, r) in rows.iter().enumerate() {
        let _ = writeln!(
            v,
            "      {}{}",
            row_json(r),
            if i + 1 < rows.len() { "," } else { "" }
        );
    }
    let _ = writeln!(v, "    }},");
    let _ = writeln!(v, "    \"legacy_240\": {{");
    for (i, r) in legacy_rows.iter().enumerate() {
        let _ = writeln!(
            v,
            "      {}{}",
            row_json(r),
            if i + 1 < legacy_rows.len() { "," } else { "" }
        );
    }
    let _ = writeln!(v, "    }},");
    let _ = writeln!(v, "    \"agrp_peak_hz\": {},", jf(nom.agrp_max_hz));
    let _ = writeln!(
        v,
        "    \"agrp_peak_open_loop_hz\": {},",
        jf(nom_open.agrp_max_hz)
    );
    let _ = writeln!(v, "    \"native_ms_per_sim_day\": {},", jf(ms_day));
    let _ = writeln!(
        v,
        "    \"anchors\": {{\"gastric_first_hour_ratio\": {}, \"gastric_first_hour_ratio_lit\": 0.73, \"misalign_glucose_ratio\": {}, \"misalign_glucose_ratio_lit\": 1.06, \"cortisol_car_peak_min\": {}, \"cortisol_car_peak_min_lit\": \"30-45\"}},",
        jf(a_gastric),
        jf(a_misalign),
        jf(a_car)
    );
    let (nt, np) = tests
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap_or(("null".into(), "null".into()));
    let _ = writeln!(v, "    \"tests\": {nt},\n    \"tests_passing\": {np},");
    let _ = writeln!(v, "    \"loc_rust\": {loc},");
    let _ = writeln!(
        v,
        "    \"coverage\": {},",
        cov.map(|x| jf(x / 100.0)).unwrap_or("null".into())
    );
    let _ = writeln!(v, "    \"w_ghr_calibrated\": {},", jf(c.w_ghr));
    let _ = writeln!(v, "    \"fit_rmse\": {},", jf(f.rmse));
    let _ = writeln!(v, "    \"fit_loo_rmse\": {},", jf(loo));
    let _ = writeln!(v, "    \"fit_targets\": {},", ts.len());
    let _ = writeln!(v, "    \"fit_free_params\": {},", fit::FIT_KEYS.len());
    let _ = writeln!(
        v,
        "    \"note\": \"Simulation only. Not a medical device. Not medical advice. fit_* are relative RMSE vs published summary values (data/SOURCES.md).\""
    );

    let sha = cmd("git", &["rev-parse", "--short", "HEAD"], &repo);
    let dirty = !cmd(
        "git",
        &["status", "--porcelain", "--untracked-files=no"],
        &repo,
    )
    .is_empty();
    let machine = format!(
        "{} {}, {} cores, {}, {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        threads,
        cmd("rustc", &["--version"], &repo),
        {
            let b = cmd("sysctl", &["-n", "machdep.cpu.brand_string"], &repo);
            if b.is_empty() {
                "cpu n/a".to_string()
            } else {
                b
            }
        }
    );
    let j = format!(
        "{{\n  \"generated_at\": \"{}\",\n  \"git_sha\": \"{}{}\",\n  \"machine\": \"{}\",\n  \"value\": {{\n{}  }}\n}}\n",
        iso_now(),
        sha,
        if dirty { "-dirty" } else { "" },
        machine,
        v
    );
    match out_path {
        Some(path) => std::fs::write(path, &j).expect("write --out"),
        None => {
            std::fs::create_dir_all(repo.join("results")).ok();
            std::fs::write(repo.join("results/metrics.json"), &j)
                .expect("write results/metrics.json");
            std::fs::write(repo.join("metrics.json"), &j).expect("write metrics.json");
        }
    }

    // fit detail
    let mut fj = String::from("{\n  \"residuals\": [\n");
    let rows_fit: Vec<String> = f
        .residuals
        .iter()
        .map(|(id, m, r)| {
            let t = ts.iter().find(|t| &t.id == id).unwrap();
            let lo = held
                .iter()
                .find(|h| &h.0 == id)
                .map(|h| h.1)
                .unwrap_or(f64::NAN);
            format!(
                "    {{\"id\": \"{id}\", \"target\": {}, \"model\": {}, \"rel_residual\": {}, \"loo_rel_residual\": {}, \"source\": \"{}\"}}",
                jf(t.value), jf(*m), jf(*r), jf(lo), t.source
            )
        })
        .collect();
    fj.push_str(&rows_fit.join(",\n"));
    let _ = write!(
        fj,
        "\n  ],\n  \"params\": {{\"lep_mean\": {}, \"lep_a1\": {}, \"lep_p1\": {}, \"lep_a2\": {}, \"ghr_rise\": {}, \"ghr_tau_s\": {}, \"glp1_tau_s\": {}, \"sleep_lep\": {}, \"sleep_ghr\": {}}}\n}}\n",
        jf(p.lep_mean), jf(p.lep_a1), jf(p.lep_p1), jf(p.lep_a2), jf(p.ghr_rise),
        jf(p.ghr_tau_s), jf(p.glp1_tau_s), jf(p.sleep_lep), jf(p.sleep_ghr)
    );
    if !args.iter().any(|a| a == "--out") {
        std::fs::write(repo.join("data/fit_result.json"), &fj).expect("write fit_result.json");
    }
    eprintln!("done in {:.1}s", t0.elapsed().as_secs_f64());
}
