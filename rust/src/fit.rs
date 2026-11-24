//! Fit hormone parameters to the published summary values in `data/targets.csv`.
//! Method: Nelder-Mead on relative residuals; leave-one-out check for honesty.

use crate::hormones::{self, Day, Meal, Params, HOUR};

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub id: String,
    pub value: f64,
    pub scale: f64,
    pub kind: String,
    pub source: String,
}

/// Parse `targets.csv` (quoted last column allowed). Returns an error string on bad rows.
pub fn parse_targets(csv: &str) -> Result<Vec<Target>, String> {
    let mut out = vec![];
    for (i, line) in csv.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.splitn(8, ',').collect();
        if f.len() < 7 {
            return Err(format!("line {}: expected 8 fields", i + 1));
        }
        let num = |s: &str| {
            s.trim()
                .parse::<f64>()
                .map_err(|e| format!("line {}: {e}", i + 1))
        };
        let t = Target {
            id: f[0].trim().to_string(),
            value: num(f[2])?,
            scale: num(f[4])?,
            kind: f[5].trim().to_string(),
            source: f[6].trim().to_string(),
        };
        if t.scale <= 0.0 {
            return Err(format!("line {}: scale must be > 0", i + 1));
        }
        out.push(t);
    }
    Ok(out)
}

pub const TARGETS_CSV: &str = include_str!("../../data/targets.csv");

pub fn targets() -> Vec<Target> {
    parse_targets(TARGETS_CSV).expect("data/targets.csv is valid")
}

/// Model-side summary values, keyed like `targets.csv`.
pub fn summaries(p: &Params) -> Vec<(&'static str, f64)> {
    let dt = 60.0;
    let std = Day::standard();
    let tr = hormones::run(p, &std, 48.0, dt);
    let day2: Vec<usize> = (0..tr.h.len()).filter(|&i| tr.h[i] >= 24.0).collect();
    let idx_at = |h: f64| ((h * HOUR / dt).round() as usize).min(tr.h.len() - 1);

    // ghrelin around each meal on day 2
    let (mut ratio, mut trough) = (0.0, 0.0);
    for m in &std.meals {
        let i0 = idx_at(24.0 + m.at_h);
        let i1 = idx_at(24.0 + m.at_h + 3.0);
        let pre = tr.s[i0].ghrelin;
        let (mut jmin, mut vmin) = (i0, f64::MAX);
        for (j, st) in tr.s.iter().enumerate().take(i1 + 1).skip(i0) {
            if st.ghrelin < vmin {
                vmin = st.ghrelin;
                jmin = j;
            }
        }
        ratio += pre / vmin;
        trough += (jmin - i0) as f64 * dt / 60.0;
    }
    let nm = std.meals.len() as f64;

    // leptin on day 2
    let (mut imax, mut imin, mut sum) = (day2[0], day2[0], 0.0);
    for &i in &day2 {
        let l = tr.s[i].leptin;
        if l > tr.s[imax].leptin {
            imax = i;
        }
        if l < tr.s[imin].leptin {
            imin = i;
        }
        sum += l;
    }
    let (lmax, lmin) = (tr.s[imax].leptin, tr.s[imin].leptin);

    // GLP-1 after breakfast
    let b0 = idx_at(24.0 + std.meals[0].at_h);
    let b1 = idx_at(24.0 + std.meals[0].at_h + 3.0);
    let mut jp = b0;
    for j in b0..=b1 {
        if tr.s[j].glp1 > tr.s[jp].glp1 {
            jp = j;
        }
    }

    // sleep restriction (4 h) vs extension (10 h): daytime 08:00-22:00 means on day 2
    let day_mean = |sleep_h: f64| {
        let d = Day {
            sleep_h,
            ..Day::standard()
        };
        let t = hormones::run(p, &d, 48.0, dt);
        let ii: Vec<usize> = (0..t.h.len())
            .filter(|&i| t.h[i] >= 32.0 && t.h[i] < 46.0)
            .collect();
        let n = ii.len() as f64;
        (
            ii.iter().map(|&i| t.s[i].leptin).sum::<f64>() / n,
            ii.iter().map(|&i| t.s[i].ghrelin).sum::<f64>() / n,
        )
    };
    let (l4, g4) = day_mean(4.0);
    let (l10, g10) = day_mean(10.0);

    vec![
        ("ghr_ratio", ratio / nm),
        ("ghr_trough", trough / nm),
        ("lep_zenith", tr.h[imax] - 24.0),
        ("lep_nadir", tr.h[imin] - 24.0),
        ("lep_amp", (lmax - lmin) / lmin * 100.0),
        ("lep_mean", sum / day2.len() as f64),
        ("glp1_tpeak", (jp - b0) as f64 * dt / 60.0),
        ("sleep_lep", l4 / l10),
        ("sleep_ghr", g4 / g10),
    ]
}

/// Circular clock difference in hours, in (-12, 12].
pub fn clock_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(24.0);
    if d > 12.0 {
        d - 24.0
    } else {
        d
    }
}

pub fn residual(t: &Target, model: f64) -> f64 {
    match t.kind.as_str() {
        "clock" => clock_diff(model, t.value) / t.scale,
        "upper_bound" => (model - t.value).max(0.0) / t.scale,
        _ => (model - t.value) / t.scale,
    }
}

/// Residuals for the given targets (skipping ids not in `use_ids` when provided).
pub fn residuals(p: &Params, ts: &[Target]) -> Vec<(String, f64, f64)> {
    let s = summaries(p);
    ts.iter()
        .filter_map(|t| {
            s.iter()
                .find(|(k, _)| *k == t.id)
                .map(|(_, m)| (t.id.clone(), *m, residual(t, *m)))
        })
        .collect()
}

pub fn rmse(r: &[f64]) -> f64 {
    if r.is_empty() {
        return 0.0;
    }
    (r.iter().map(|x| x * x).sum::<f64>() / r.len() as f64).sqrt()
}

/// Fitted parameter names. 9 free parameters against 9 targets: the in-sample fit is
/// close to exactly determined, so the leave-one-out error is the honest number.
pub const FIT_KEYS: [&str; 9] = [
    "lep_mean",
    "lep_a1",
    "lep_p1",
    "lep_a2",
    "ghr_rise",
    "ghr_tau_s",
    "glp1_tau_s",
    "sleep_lep",
    "sleep_ghr",
];

/// Map an unconstrained vector z to parameters (positive via exp, bounded via clamp).
pub fn apply(base: &Params, z: &[f64]) -> Params {
    let mut p = base.clone();
    let e = |i: usize| z.get(i).copied().unwrap_or(0.0).clamp(-4.0, 4.0).exp();
    p.lep_mean = base.lep_mean * e(0);
    p.lep_a1 = (base.lep_a1 * e(1)).clamp(0.0, 0.8);
    p.lep_p1 = (base.lep_p1 + 3.0 * z.get(2).copied().unwrap_or(0.0)).rem_euclid(24.0);
    p.lep_a2 = (base.lep_a2 * e(3)).clamp(0.0, 0.4);
    p.ghr_rise = (base.ghr_rise * e(4)).clamp(0.0, 5.0);
    p.ghr_tau_s = (base.ghr_tau_s * e(5)).clamp(60.0, 4.0 * HOUR);
    p.glp1_tau_s = (base.glp1_tau_s * e(6)).clamp(60.0, 4.0 * HOUR);
    p.sleep_lep = (base.sleep_lep * e(7)).clamp(0.0, 0.9);
    p.sleep_ghr = (base.sleep_ghr * e(8)).clamp(0.0, 3.0);
    p
}

/// Minimal Nelder-Mead minimiser.
pub fn nelder_mead<F: FnMut(&[f64]) -> f64>(
    mut f: F,
    x0: &[f64],
    step: f64,
    iters: usize,
) -> (Vec<f64>, f64) {
    let n = x0.len();
    let mut pts: Vec<Vec<f64>> = vec![x0.to_vec()];
    for i in 0..n {
        let mut x = x0.to_vec();
        x[i] += step;
        pts.push(x);
    }
    let mut vals: Vec<f64> = pts.iter().map(|x| f(x)).collect();
    for _ in 0..iters {
        let mut ord: Vec<usize> = (0..=n).collect();
        ord.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
        pts = ord.iter().map(|&i| pts[i].clone()).collect();
        vals = ord.iter().map(|&i| vals[i]).collect();
        if (vals[n] - vals[0]).abs() < 1e-12 {
            break;
        }
        let cen: Vec<f64> = (0..n)
            .map(|j| pts[..n].iter().map(|p| p[j]).sum::<f64>() / n as f64)
            .collect();
        let lerp = |a: &[f64], t: f64| -> Vec<f64> {
            (0..n).map(|j| cen[j] + t * (a[j] - cen[j])).collect()
        };
        let xr = lerp(&pts[n], -1.0);
        let fr = f(&xr);
        if fr < vals[0] {
            let xe = lerp(&pts[n], -2.0);
            let fe = f(&xe);
            if fe < fr {
                pts[n] = xe;
                vals[n] = fe;
            } else {
                pts[n] = xr;
                vals[n] = fr;
            }
        } else if fr < vals[n - 1] {
            pts[n] = xr;
            vals[n] = fr;
        } else {
            let xc = lerp(&pts[n], 0.5);
            let fc = f(&xc);
            if fc < vals[n] {
                pts[n] = xc;
                vals[n] = fc;
            } else {
                for i in 1..=n {
                    pts[i] = (0..n)
                        .map(|j| pts[0][j] + 0.5 * (pts[i][j] - pts[0][j]))
                        .collect();
                    vals[i] = f(&pts[i]);
                }
            }
        }
    }
    let mut b = 0;
    for i in 1..=n {
        if vals[i] < vals[b] {
            b = i;
        }
    }
    (pts[b].clone(), vals[b])
}

#[derive(Clone, Debug)]
pub struct FitResult {
    pub params: Params,
    pub rmse: f64,
    pub residuals: Vec<(String, f64, f64)>,
}

/// Fit to `ts` starting from `base`.
pub fn fit(base: &Params, ts: &[Target], iters: usize) -> FitResult {
    let obj = |z: &[f64]| {
        let r: Vec<f64> = residuals(&apply(base, z), ts).iter().map(|x| x.2).collect();
        let v = rmse(&r);
        if v.is_finite() {
            v
        } else {
            1e9
        }
    };
    let (z, _) = nelder_mead(obj, &vec![0.0; FIT_KEYS.len()], 0.3, iters);
    let params = apply(base, &z);
    let res = residuals(&params, ts);
    let r: Vec<f64> = res.iter().map(|x| x.2).collect();
    FitResult {
        rmse: rmse(&r),
        params,
        residuals: res,
    }
}

/// Leave-one-out: fit on all targets but one, score the held-out one. Returns RMSE of those.
pub fn leave_one_out(base: &Params, ts: &[Target], iters: usize) -> (f64, Vec<(String, f64)>) {
    let mut held = vec![];
    for k in 0..ts.len() {
        let train: Vec<Target> = ts
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != k)
            .map(|(_, t)| t.clone())
            .collect();
        let f = fit(base, &train, iters);
        let r = residuals(&f.params, std::slice::from_ref(&ts[k]));
        held.push((ts[k].id.clone(), r.first().map(|x| x.2).unwrap_or(f64::NAN)));
    }
    let v: Vec<f64> = held.iter().map(|x| x.1).collect();
    (rmse(&v), held)
}

/// A single test meal on an otherwise fasting day (helper for tests).
pub fn single_meal_day(carbs_g: f64) -> Day {
    Day {
        meals: vec![Meal { at_h: 8.0, carbs_g }],
        doses: vec![],
        sleep_h: 8.0,
    }
}
