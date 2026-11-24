//! Closed-loop 24-h simulation: hormones -> neural drive -> AgRP/POMC plant -> controller.
//! Plus the stress suite (240 scenarios) and the pass rule.

use crate::control::{Plant, Rng};
use crate::controllers::Controller;
use crate::hormones::{self, Coupling, Day, Dose, Ext, Meal, Params, State, HOUR};

#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub category: &'static str,
    /// inputs for the warm-up day (hormones only)
    pub warm: Day,
    /// inputs for the measured day (closed loop)
    pub day: Day,
    pub gain_err: [f64; 2],
    pub noise: f64,
    pub seed: u64,
    /// circadian displacement, stress, illness (both days)
    pub ext: Ext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayResult {
    pub agrp_max_hz: f64,
    pub agrp_mean_hz: f64,
    pub frac_agrp_below_2hz: f64,
    pub pomc_mean_hz: f64,
    pub u_peak: [f64; 2],
    pub u_mean: [f64; 2],
    pub drive_agrp_max_hz: f64,
    pub finite: bool,
    pub bounded: bool,
}

impl DayResult {
    /// Pass rule: AgRP < 2 Hz for >= 95% of the day, POMC mean within 1 Hz of 8 Hz,
    /// every state finite and bounded.
    pub fn pass(&self) -> bool {
        self.finite
            && self.bounded
            && self.frac_agrp_below_2hz >= 0.95
            && (self.pomc_mean_hz - 8.0).abs() < 1.0
    }
}

pub const DT: f64 = 1.0;
pub const SETTLE_S: f64 = 120.0;
pub const X_BOUND: f64 = 200.0;

/// Run one scenario. `closed = false` runs the open loop (u = 0).
pub fn run_day(
    sc: &Scenario,
    ctrl: Option<&mut dyn Controller>,
    p: &Params,
    c: &Coupling,
) -> DayResult {
    // warm-up day, hormones only, coarse step
    let mut s = State::basal(p);
    let wdt = 60.0;
    let n_w = (24.0 * HOUR / wdt) as usize;
    for k in 0..n_w {
        let h = k as f64 * wdt / HOUR;
        hormones::apply_events(&mut s, &sc.warm, h, wdt);
        hormones::step_ext(&mut s, p, h, wdt, sc.warm.sleep_h, &sc.ext);
    }
    let plant = Plant::default();
    let g = [
        plant.gain[0] * sc.gain_err[0],
        plant.gain[1] * sc.gain_err[1],
    ];
    let mut rng = Rng(sc.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut x = hormones::neural_drive(&s, p, c);
    let mut yf = x;
    let n = (24.0 * HOUR / DT) as usize;
    let mut ctrl = ctrl;
    if let Some(cc) = ctrl.as_deref_mut() {
        cc.reset();
    }
    let mut r = DayResult {
        agrp_max_hz: f64::MIN,
        agrp_mean_hz: 0.0,
        frac_agrp_below_2hz: 0.0,
        pomc_mean_hz: 0.0,
        u_peak: [0.0; 2],
        u_mean: [0.0; 2],
        drive_agrp_max_hz: f64::MIN,
        finite: true,
        bounded: true,
    };
    let (mut cnt, mut below) = (0usize, 0usize);
    for k in 0..n {
        let t = k as f64 * DT;
        let h = 24.0 + t / HOUR;
        hormones::apply_events(&mut s, &sc.day, h, DT);
        hormones::step_ext(&mut s, p, h, DT, sc.day.sleep_h, &sc.ext);
        let d = hormones::neural_drive(&s, p, c);
        let y = [
            x[0] + sc.noise * rng.normal(),
            x[1] + sc.noise * rng.normal(),
        ];
        yf = [
            yf[0] + (y[0] - yf[0]) * DT.min(1.0),
            yf[1] + (y[1] - yf[1]) * DT.min(1.0),
        ];
        let u = match ctrl.as_deref_mut() {
            Some(cc) => cc.step(yf, DT, plant.u_max),
            None => [0.0; 2],
        };
        for i in 0..2 {
            x[i] = (x[i] + DT * (d[i] + g[i] * u[i] - x[i]) / plant.tau).max(0.0);
            r.u_peak[i] = r.u_peak[i].max(u[i]);
            r.u_mean[i] += u[i];
            if !(u[i] >= 0.0 && u[i] <= plant.u_max[i]) {
                r.bounded = false;
            }
        }
        if !x[0].is_finite() || !x[1].is_finite() || !s.is_finite() {
            r.finite = false;
            break;
        }
        if x[0] > X_BOUND || x[1] > X_BOUND {
            r.bounded = false;
        }
        r.drive_agrp_max_hz = r.drive_agrp_max_hz.max(d[0]);
        if t >= SETTLE_S {
            cnt += 1;
            r.agrp_max_hz = r.agrp_max_hz.max(x[0]);
            r.agrp_mean_hz += x[0];
            r.pomc_mean_hz += x[1];
            if x[0] < 2.0 {
                below += 1;
            }
        }
    }
    let cn = cnt.max(1) as f64;
    r.agrp_mean_hz /= cn;
    r.pomc_mean_hz /= cn;
    r.frac_agrp_below_2hz = below as f64 / cn;
    r.u_mean = [r.u_mean[0] / n as f64, r.u_mean[1] / n as f64];
    r
}

// ---------------------------------------------------------------- suite

fn meal(at_h: f64, carbs_g: f64) -> Meal {
    Meal { at_h, carbs_g }
}

/// Eight meal patterns: (name, category, meals).
pub fn meal_patterns() -> Vec<(&'static str, &'static str, Vec<Meal>)> {
    let std = Day::standard().meals;
    vec![
        ("standard", "nominal", std.clone()),
        ("skip_breakfast", "fasting", std[1..].to_vec()),
        ("skip_lunch", "fasting", vec![std[0], std[2]]),
        ("fast_24h", "fasting", vec![]),
        (
            "binge_dinner",
            "binge",
            vec![std[0], std[1], meal(18.5, 220.0)],
        ),
        ("binge_night", "binge", {
            let mut m = std.clone();
            m.push(meal(23.0, 150.0));
            m
        }),
        (
            "grazing",
            "binge",
            (0..8).map(|i| meal(8.0 + 2.0 * i as f64, 35.0)).collect(),
        ),
        (
            "big_breakfast",
            "binge",
            vec![meal(8.0, 160.0), std[1], std[2]],
        ),
    ]
}

/// Five sleep conditions (hours in bed).
pub const SLEEP: [(&str, f64); 5] = [
    ("sleep8", 8.0),
    ("sleep10", 10.0),
    ("sleep6", 6.0),
    ("sleep4", 4.0),
    ("allnighter", 0.0),
];

/// Six dose regimes: (name, warm-up doses, measured-day doses).
pub fn dose_regimes() -> Vec<(&'static str, Vec<Dose>, Vec<Dose>)> {
    let d = |u: f64| {
        vec![Dose {
            at_h: 8.0,
            units: u,
        }]
    };
    vec![
        ("nodose", vec![], vec![]),
        ("dose1", d(1.0), d(1.0)),
        ("dose2", d(2.0), d(2.0)),
        ("start", vec![], d(1.0)),
        ("stop", d(1.0), vec![]),
        ("double", d(1.0), d(2.0)),
    ]
}

/// 8 meal patterns x 5 sleep x 6 dose = 240 scenarios. Each gets a seeded plant-gain error
/// in [0.7, 1.3] per channel and 0.3 Hz sensor noise.
pub fn suite() -> Vec<Scenario> {
    let mut rng = Rng(0xC0FF_EE11 | 1);
    let mut out = vec![];
    for (mn, cat, meals) in meal_patterns() {
        for (sn, sleep_h) in SLEEP {
            for (dn, dw, dd) in dose_regimes() {
                let category = if sleep_h < 8.0 && cat == "nominal" {
                    "sleep_loss"
                } else if dn != "nodose" && cat == "nominal" && sleep_h >= 8.0 {
                    "dose"
                } else {
                    cat
                };
                let ge = [0.7 + 0.6 * rng.next_f64(), 0.7 + 0.6 * rng.next_f64()];
                let seed = out.len() as u64 + 1;
                out.push(Scenario {
                    name: format!("{mn}/{sn}/{dn}"),
                    category,
                    warm: Day {
                        meals: meals.clone(),
                        doses: dw.clone(),
                        sleep_h: 8.0,
                    },
                    day: Day {
                        meals: meals.clone(),
                        doses: dd,
                        sleep_h,
                    },
                    gain_err: ge,
                    noise: 0.3,
                    seed,
                    ext: Ext::default(),
                });
            }
        }
    }
    out
}

/// Nominal scenario: standard meals, 8 h sleep, no drug, no model error, no noise.
pub fn nominal() -> Scenario {
    Scenario {
        name: "nominal".into(),
        category: "nominal",
        warm: Day::standard(),
        day: Day::standard(),
        gain_err: [1.0, 1.0],
        noise: 0.0,
        seed: 1,
        ext: Ext::default(),
    }
}

#[derive(Clone, Debug)]
pub struct SuiteReport {
    pub controller: String,
    pub n: usize,
    pub passed: usize,
    pub worst_agrp_hz: f64,
    pub mean_frac_below_2hz: f64,
    pub mean_pomc_err_hz: f64,
    pub mean_u_agrp: f64,
    pub failures: Vec<String>,
}

impl SuiteReport {
    pub fn pass_rate(&self) -> f64 {
        self.passed as f64 / self.n.max(1) as f64
    }
}

pub fn run_suite(name: &str, scenarios: &[Scenario], p: &Params, c: &Coupling) -> SuiteReport {
    let mut rep = SuiteReport {
        controller: name.to_string(),
        n: scenarios.len(),
        passed: 0,
        worst_agrp_hz: 0.0,
        mean_frac_below_2hz: 0.0,
        mean_pomc_err_hz: 0.0,
        mean_u_agrp: 0.0,
        failures: vec![],
    };
    for sc in scenarios {
        let r = match crate::controllers::by_name(name, sc.seed) {
            Some(mut cc) => run_day(sc, Some(cc.as_mut()), p, c),
            None => run_day(sc, None, p, c),
        };
        if r.pass() {
            rep.passed += 1;
        } else {
            rep.failures.push(sc.name.clone());
        }
        rep.worst_agrp_hz = rep.worst_agrp_hz.max(r.agrp_max_hz);
        rep.mean_frac_below_2hz += r.frac_agrp_below_2hz;
        rep.mean_pomc_err_hz += (r.pomc_mean_hz - 8.0).abs();
        rep.mean_u_agrp += r.u_mean[0];
    }
    let n = rep.n.max(1) as f64;
    rep.mean_frac_below_2hz /= n;
    rep.mean_pomc_err_hz /= n;
    rep.mean_u_agrp /= n;
    rep
}

/// Calibrate the ghrelin -> AgRP weight so the open-loop nominal day peaks at `peak_hz`.
/// v0.1 used a 30 Hz pre-meal hunger peak; this keeps the new model on the same scale.
/// Bisection on `w_ghr` in [0, 400]. Returns the calibrated coupling.
pub fn calibrate_coupling(p: &Params, base: &Coupling, peak_hz: f64) -> Coupling {
    let sc = nominal();
    let peak = |w: f64| {
        let c = Coupling {
            w_ghr: w,
            ..base.clone()
        };
        run_day(&sc, None, p, &c).agrp_max_hz
    };
    let (mut lo, mut hi) = (0.0, 400.0);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if peak(mid) < peak_hz {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Coupling {
        w_ghr: 0.5 * (lo + hi),
        ..base.clone()
    }
}

// ---------------------------------------------------------------- 1,000-scenario stress suite

/// Number of scenarios in the v1.0 suite.
pub const N_STRESS: usize = 1000;
/// Seed of the scenario generator. The suite is fully determined by this seed.
pub const STRESS_SEED: u64 = 0x6AD3_2026;

pub const STRESS_KINDS: [&str; 5] = [
    "shift_work",
    "illness",
    "travel",
    "missed_meals",
    "sensor_noise",
];

fn uni(r: &mut Rng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * r.next_f64()
}

fn shifted(meals: &[Meal], by_h: f64) -> Vec<Meal> {
    meals
        .iter()
        .map(|m| Meal {
            at_h: (m.at_h + by_h).rem_euclid(24.0),
            carbs_g: m.carbs_g,
        })
        .collect()
}

/// One random scenario of the given kind (index into [`STRESS_KINDS`]).
pub fn stress_scenario(kind: usize, idx: usize, r: &mut Rng) -> Scenario {
    let std = Day::standard().meals;
    let mut meals = std.clone();
    let mut sleep_h = uni(r, 6.0, 9.0);
    let mut ext = Ext::default();
    let mut noise = 0.3;
    match kind {
        0 => {
            // shift work: meals and sleep move toward night; the body clock follows only in part
            let by = uni(r, 8.0, 14.0);
            meals = shifted(&std, by);
            sleep_h = uni(r, 4.0, 7.0);
            ext.circ_shift_h = by * uni(r, 0.4, 0.9) * if by > 12.0 { -1.0 } else { 1.0 };
            ext.stress = uni(r, 0.0, 0.4);
        }
        1 => {
            // illness: stress and illness raise cortisol and lower the ghrelin target
            ext.illness = uni(r, 0.3, 1.0);
            ext.stress = uni(r, 0.2, 0.8);
            sleep_h = uni(r, 5.0, 10.0);
            if r.next_f64() < 0.5 {
                meals.remove((r.next_f64() * meals.len() as f64) as usize % meals.len());
            }
        }
        2 => {
            // travel: time-zone shift of 3 to 9 h, east (+) or west (-)
            let z = uni(r, 3.0, 9.0);
            ext.circ_shift_h = if r.next_f64() < 0.5 { z } else { -z };
            sleep_h = uni(r, 4.0, 8.0);
            meals = shifted(&std, uni(r, -2.0, 2.0));
        }
        3 => {
            // missed meals: drop 1 to 3 meals, or a full fast; the next meal may be larger
            if r.next_f64() < 0.12 {
                meals.clear();
            } else {
                let drop = 1 + (r.next_f64() * 2.0) as usize;
                for _ in 0..drop.min(meals.len().saturating_sub(1)) {
                    let k = (r.next_f64() * meals.len() as f64) as usize % meals.len();
                    meals.remove(k);
                }
                if r.next_f64() < 0.5 {
                    if let Some(m) = meals.last_mut() {
                        m.carbs_g *= uni(r, 1.2, 2.0);
                    }
                }
            }
        }
        _ => {
            // sensor noise from 0.5 to 2 Hz (log-uniform)
            noise = (0.5f64.ln() + (2.0f64.ln() - 0.5f64.ln()) * r.next_f64()).exp();
            meals = shifted(&std, uni(r, -1.0, 1.0));
        }
    }
    let dose: Vec<Dose> = if r.next_f64() < 0.3 {
        vec![Dose {
            at_h: 8.0,
            units: uni(r, 0.5, 2.0),
        }]
    } else {
        vec![]
    };
    let ge = [uni(r, 0.7, 1.3), uni(r, 0.7, 1.3)];
    Scenario {
        name: format!("{}/{:04}", STRESS_KINDS[kind], idx),
        category: STRESS_KINDS[kind],
        warm: Day {
            meals: meals.clone(),
            doses: dose.clone(),
            sleep_h: 8.0,
        },
        day: Day {
            meals,
            doses: dose,
            sleep_h,
        },
        gain_err: ge,
        noise,
        seed: idx as u64 + 1,
        ext,
    }
}

/// `n` seeded scenarios, spread evenly over the five kinds. Same `(n, seed)` gives the same list.
pub fn stress_suite(n: usize, seed: u64) -> Vec<Scenario> {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    (0..n)
        .map(|i| stress_scenario(i % STRESS_KINDS.len(), i, &mut r))
        .collect()
}

/// The v1.0 suite: [`N_STRESS`] scenarios.
pub fn suite_1000() -> Vec<Scenario> {
    stress_suite(N_STRESS, STRESS_SEED)
}

/// Wilson 95% interval for a proportion `k / n`. Returns (low, high).
pub fn wilson_ci(k: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let (z, n_f, p) = (1.959_963_984_540_054, n as f64, k as f64 / n as f64);
    let den = 1.0 + z * z / n_f;
    let centre = (p + z * z / (2.0 * n_f)) / den;
    let half = z * (p * (1.0 - p) / n_f + z * z / (4.0 * n_f * n_f)).sqrt() / den;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// Per-scenario outcome kept by the parallel runner.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub passed: bool,
    pub kind: &'static str,
    pub result: DayResult,
}

/// Run one controller over all scenarios on `threads` threads. Order of the result = order of input.
pub fn run_suite_par(
    name: &str,
    scenarios: &[Scenario],
    p: &Params,
    c: &Coupling,
    threads: usize,
) -> Vec<Outcome> {
    let threads = threads.max(1).min(scenarios.len().max(1));
    let chunk = scenarios.len().div_ceil(threads).max(1);
    let mut out: Vec<Vec<Outcome>> = vec![];
    std::thread::scope(|sc| {
        let hs: Vec<_> = scenarios
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    part.iter()
                        .map(|s| {
                            let r = match crate::controllers::by_name(name, s.seed) {
                                Some(mut cc) => run_day(s, Some(cc.as_mut()), p, c),
                                None => run_day(s, None, p, c),
                            };
                            Outcome {
                                passed: r.pass(),
                                kind: s.category,
                                result: r,
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in hs {
            out.push(h.join().expect("worker thread"));
        }
    });
    out.into_iter().flatten().collect()
}
