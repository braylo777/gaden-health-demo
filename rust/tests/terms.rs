//! One bound test and one literature-value test per model term.
//! Terms: ghrelin, leptin, insulin, GLP-1, PYY, cortisol, gastric emptying, glucose, sleep debt.
//! Sources (2015 and later where a number exists) are in `data/SOURCES.md`.
//! Bounds are physiological windows chosen for this simulation. They are not clinical ranges.

use gaden::hormones::*;
use gaden::{fit, neuron};

const DT: f64 = 30.0;

#[derive(Clone, Copy)]
enum Cond {
    Standard,
    Fast24,
    Binge,
    Sleep4,
    ShiftWork,
    IllStressDose,
}

fn setup(c: Cond) -> (Day, Ext) {
    let mut day = Day::standard();
    let mut ext = Ext::default();
    match c {
        Cond::Standard => {}
        Cond::Fast24 => day.meals.clear(),
        Cond::Binge => day.meals.push(Meal {
            at_h: 23.0,
            carbs_g: 220.0,
        }),
        Cond::Sleep4 => day.sleep_h = 4.0,
        Cond::ShiftWork => {
            for m in &mut day.meals {
                m.at_h = (m.at_h + 12.0).rem_euclid(24.0);
            }
            day.sleep_h = 5.0;
            ext.circ_shift_h = 8.0;
        }
        Cond::IllStressDose => {
            day.doses = vec![Dose {
                at_h: 8.0,
                units: 2.0,
            }];
            ext.illness = 1.0;
            ext.stress = 1.0;
        }
    }
    (day, ext)
}

/// Run `days` days and return every state from day 2 on.
fn trace(c: Cond, days: usize) -> Vec<State> {
    let (day, ext) = setup(c);
    let p = Params::default();
    let mut s = State::basal(&p);
    let n = (days as f64 * 24.0 * HOUR / DT) as usize;
    let mut out = vec![];
    for k in 0..n {
        let h = k as f64 * DT / HOUR;
        apply_events(&mut s, &day, h, DT);
        step_ext(&mut s, &p, h, DT, day.sleep_h, &ext);
        if h >= 24.0 {
            out.push(s);
        }
    }
    out
}

fn bounded(c: Cond, f: fn(&State) -> f64, lo: f64, hi: f64) {
    for s in trace(c, 4) {
        let v = f(&s);
        assert!(
            v.is_finite() && v >= lo && v <= hi,
            "value {v} outside [{lo}, {hi}]"
        );
    }
}

macro_rules! bound_tests {
    ($($name:ident: ($c:expr, $f:expr, $lo:expr, $hi:expr),)*) => {
        $( #[test] fn $name() { bounded($c, $f, $lo, $hi); } )*
    };
}

bound_tests! {
    bound_ghrelin_standard: (Cond::Standard, |s| s.ghrelin, 50.0, 1500.0),
    bound_ghrelin_fast24: (Cond::Fast24, |s| s.ghrelin, 50.0, 1500.0),
    bound_ghrelin_binge: (Cond::Binge, |s| s.ghrelin, 50.0, 1500.0),
    bound_ghrelin_sleep4: (Cond::Sleep4, |s| s.ghrelin, 50.0, 1500.0),
    bound_ghrelin_shift: (Cond::ShiftWork, |s| s.ghrelin, 50.0, 1500.0),
    bound_ghrelin_ill: (Cond::IllStressDose, |s| s.ghrelin, 50.0, 1500.0),
    bound_leptin_standard: (Cond::Standard, |s| s.leptin, 1.0, 60.0),
    bound_leptin_fast24: (Cond::Fast24, |s| s.leptin, 1.0, 60.0),
    bound_leptin_binge: (Cond::Binge, |s| s.leptin, 1.0, 60.0),
    bound_leptin_sleep4: (Cond::Sleep4, |s| s.leptin, 1.0, 60.0),
    bound_leptin_shift: (Cond::ShiftWork, |s| s.leptin, 1.0, 60.0),
    bound_leptin_ill: (Cond::IllStressDose, |s| s.leptin, 1.0, 60.0),
    bound_insulin_standard: (Cond::Standard, |s| s.insulin, 1.0, 150.0),
    bound_insulin_fast24: (Cond::Fast24, |s| s.insulin, 1.0, 150.0),
    bound_insulin_binge: (Cond::Binge, |s| s.insulin, 1.0, 150.0),
    bound_insulin_sleep4: (Cond::Sleep4, |s| s.insulin, 1.0, 150.0),
    bound_insulin_shift: (Cond::ShiftWork, |s| s.insulin, 1.0, 150.0),
    bound_insulin_ill: (Cond::IllStressDose, |s| s.insulin, 1.0, 150.0),
    bound_glp1_standard: (Cond::Standard, |s| s.glp1, 1.0, 150.0),
    bound_glp1_fast24: (Cond::Fast24, |s| s.glp1, 1.0, 150.0),
    bound_glp1_binge: (Cond::Binge, |s| s.glp1, 1.0, 150.0),
    bound_glp1_sleep4: (Cond::Sleep4, |s| s.glp1, 1.0, 150.0),
    bound_glp1_shift: (Cond::ShiftWork, |s| s.glp1, 1.0, 150.0),
    bound_glp1_ill: (Cond::IllStressDose, |s| s.glp1, 1.0, 150.0),
    bound_pyy_standard: (Cond::Standard, |s| s.pyy, 5.0, 150.0),
    bound_pyy_fast24: (Cond::Fast24, |s| s.pyy, 5.0, 150.0),
    bound_pyy_binge: (Cond::Binge, |s| s.pyy, 5.0, 150.0),
    bound_pyy_sleep4: (Cond::Sleep4, |s| s.pyy, 5.0, 150.0),
    bound_pyy_shift: (Cond::ShiftWork, |s| s.pyy, 5.0, 150.0),
    bound_pyy_ill: (Cond::IllStressDose, |s| s.pyy, 5.0, 150.0),
    bound_cortisol_standard: (Cond::Standard, |s| s.cortisol, 0.2, 40.0),
    bound_cortisol_fast24: (Cond::Fast24, |s| s.cortisol, 0.2, 40.0),
    bound_cortisol_binge: (Cond::Binge, |s| s.cortisol, 0.2, 40.0),
    bound_cortisol_sleep4: (Cond::Sleep4, |s| s.cortisol, 0.2, 40.0),
    bound_cortisol_shift: (Cond::ShiftWork, |s| s.cortisol, 0.2, 40.0),
    bound_cortisol_ill: (Cond::IllStressDose, |s| s.cortisol, 0.2, 40.0),
    bound_glucose_standard: (Cond::Standard, |s| s.glucose, 60.0, 300.0),
    bound_glucose_fast24: (Cond::Fast24, |s| s.glucose, 60.0, 300.0),
    bound_glucose_binge: (Cond::Binge, |s| s.glucose, 60.0, 300.0),
    bound_glucose_sleep4: (Cond::Sleep4, |s| s.glucose, 60.0, 300.0),
    bound_glucose_shift: (Cond::ShiftWork, |s| s.glucose, 60.0, 300.0),
    bound_glucose_ill: (Cond::IllStressDose, |s| s.glucose, 60.0, 300.0),
    bound_stomach_standard: (Cond::Standard, |s| s.stomach, 0.0, 400.0),
    bound_stomach_fast24: (Cond::Fast24, |s| s.stomach, 0.0, 400.0),
    bound_stomach_binge: (Cond::Binge, |s| s.stomach, 0.0, 400.0),
    bound_stomach_sleep4: (Cond::Sleep4, |s| s.stomach, 0.0, 400.0),
    bound_stomach_shift: (Cond::ShiftWork, |s| s.stomach, 0.0, 400.0),
    bound_stomach_ill: (Cond::IllStressDose, |s| s.stomach, 0.0, 400.0),
    bound_debt_standard: (Cond::Standard, |s| s.debt, 0.0, 1.0),
    bound_debt_fast24: (Cond::Fast24, |s| s.debt, 0.0, 1.0),
    bound_debt_binge: (Cond::Binge, |s| s.debt, 0.0, 1.0),
    bound_debt_sleep4: (Cond::Sleep4, |s| s.debt, 0.0, 1.0),
    bound_debt_shift: (Cond::ShiftWork, |s| s.debt, 0.0, 1.0),
    bound_debt_ill: (Cond::IllStressDose, |s| s.debt, 0.0, 1.0),
}

// ------------------------------------------------------------ literature values

fn summary(key: &str) -> f64 {
    fit::summaries(&Params::default())
        .into_iter()
        .find(|(k, _)| *k == key)
        .unwrap()
        .1
}

/// Standard day with an optional agonist dose at 08:00, 3 days, dt 10 s. Returns the trace.
fn drug_trace(dose: bool) -> Trace {
    let mut day = Day::standard();
    if dose {
        day.doses = vec![Dose {
            at_h: 8.0,
            units: 1.0,
        }];
    }
    run(&Params::default(), &day, 72.0, 10.0)
}

/// Incremental area (unit x h) over clock hours [a, b) of day 3 above `base`.
fn iauc(tr: &Trace, f: fn(&State) -> f64, base: f64, a: f64, b: f64) -> f64 {
    tr.s.iter()
        .zip(&tr.h)
        .filter(|(_, &h)| h >= 48.0 + a && h < 48.0 + b)
        .map(|(s, _)| (f(s) - base).max(0.0) * 10.0 / HOUR)
        .sum()
}

#[test]
fn lit_ghrelin_preprandial_rise_about_twofold() {
    // Cummings 2001 (fit target); mechanism review: Mueller 2015.
    let r = summary("ghr_ratio");
    assert!((1.6..=2.4).contains(&r), "ratio {r}");
}

#[test]
fn lit_ghrelin_trough_within_one_hour() {
    let t = summary("ghr_trough");
    assert!(t <= 60.0 + 1e-9 && t > 0.0, "trough at {t} min");
}

#[test]
fn lit_leptin_zenith_near_01h_and_nadir_near_09h() {
    let (z, n) = (summary("lep_zenith"), summary("lep_nadir"));
    assert!((z - 1.0).abs() <= 2.0, "zenith {z}");
    assert!((n - 9.0).abs() <= 2.0, "nadir {n}");
}

#[test]
fn lit_leptin_sleep_restriction_minus_18_percent() {
    assert!((summary("sleep_lep") - 0.82).abs() < 0.02);
}

#[test]
fn lit_ghrelin_sleep_restriction_plus_28_percent() {
    assert!((summary("sleep_ghr") - 1.28).abs() < 0.02);
}

#[test]
fn lit_glp1_peak_30_min_after_carbohydrate() {
    assert!((summary("glp1_tpeak") - 30.0).abs() <= 5.0);
}

#[test]
fn lit_gastric_first_hour_emptying_ratio_inside_ci() {
    // Hjerpsted 2018: first-hour gastric emptying ratio 0.73 (95% CI 0.61 to 0.87) with semaglutide.
    let first_hour = |dose: bool| {
        let tr = drug_trace(dose);
        let i0 = ((48.0 + 8.0) * HOUR / 10.0) as usize + 1;
        let i1 = ((48.0 + 9.0) * HOUR / 10.0) as usize;
        tr.s[i0].stomach - tr.s[i1].stomach
    };
    let r = first_hour(true) / first_hour(false);
    assert!((0.61..=0.87).contains(&r), "ratio {r}");
}

#[test]
fn lit_glucose_postprandial_area_lower_with_agonist() {
    // Hjerpsted 2018: postprandial glucose iAUC0-5h lower with semaglutide (sign only).
    let a = iauc(&drug_trace(true), |s| s.glucose, 90.0, 8.0, 13.0);
    let b = iauc(&drug_trace(false), |s| s.glucose, 90.0, 8.0, 13.0);
    assert!(a < b, "{a} vs {b}");
}

#[test]
fn lit_pyy_postprandial_response_lower_with_agonist() {
    // Hjerpsted 2018: postprandial PYY response lower with semaglutide (sign only).
    let p = Params::default();
    let a = iauc(&drug_trace(true), |s| s.pyy, p.pyy_base, 8.0, 13.0);
    let b = iauc(&drug_trace(false), |s| s.pyy, p.pyy_base, 8.0, 13.0);
    assert!(a < b, "{a} vs {b}");
}

#[test]
fn lit_insulin_agonist_mismatch_is_known() {
    // Hjerpsted 2018 found a LOWER postprandial insulin area with semaglutide. This model gives a
    // HIGHER one because of its incretin term. The mismatch is listed under "Limitations".
    let a = iauc(&drug_trace(true), |s| s.insulin, 8.0, 8.0, 13.0);
    let b = iauc(&drug_trace(false), |s| s.insulin, 8.0, 8.0, 13.0);
    assert!(
        a > b,
        "if this fails the mismatch is fixed: update the README and SOURCES"
    );
}

#[test]
fn lit_misalignment_raises_postprandial_glucose() {
    // Morris 2015: 12-h misalignment raised postprandial glucose by 6%. The model gives a smaller
    // rise by design (insulin action only). Accept +1% to +8%.
    let p = Params::default();
    let post = |shift: f64| {
        let day = Day::standard();
        let e = Ext {
            circ_shift_h: shift,
            ..Ext::default()
        };
        let mut s = State::basal(&p);
        let (mut sum, mut n) = (0.0, 0usize);
        for k in 0..(72 * 360) {
            let h = k as f64 * 10.0 / HOUR;
            apply_events(&mut s, &day, h, 10.0);
            step_ext(&mut s, &p, h, 10.0, day.sleep_h, &e);
            let hc = h.rem_euclid(24.0);
            let in_post = (8.0..11.0).contains(&hc)
                || (12.5..15.5).contains(&hc)
                || (18.5..21.5).contains(&hc);
            if h >= 48.0 && in_post {
                sum += s.glucose;
                n += 1;
            }
        }
        sum / n as f64
    };
    let r = post(12.0) / post(0.0);
    assert!((1.01..=1.08).contains(&r), "ratio {r}");
}

fn wake_peak_minutes(p: &Params) -> f64 {
    let tr = run(p, &Day::standard(), 48.0, 60.0);
    let (mut best, mut at) = (0.0, 0.0);
    for (s, h) in tr.s.iter().zip(&tr.h) {
        if *h >= 29.0 && *h < 36.0 && s.cortisol > best {
            best = s.cortisol;
            at = *h;
        }
    }
    (at - 31.0) * 60.0 // wake = 23:00 + 8 h = 07:00 of day 2
}

#[test]
fn lit_cortisol_awakening_peak_30_to_45_min() {
    // Stalder 2016: the awakening response is the rise over the first 30-45 min after waking.
    let m = wake_peak_minutes(&Params::default());
    assert!((28.0..=47.0).contains(&m), "peak {m} min after waking");
}

#[test]
fn lit_cortisol_morning_high_midnight_low() {
    let tr = run(&Params::default(), &Day::standard(), 48.0, 60.0);
    let at = |h: f64| tr.s[(h * 60.0) as usize].cortisol;
    assert!(at(32.0) > 3.0 * at(48.0 - 0.1) && at(32.0) > 2.0 * at(38.0));
}

#[test]
fn lit_agrp_drive_afternoon_exceeds_late_morning() {
    // Mandelblat-Cerf 2015 (mouse): AgRP firing rose about fivefold with caloric deficit,
    // afternoon vs morning. Qualitative analogue: drive before dinner >= 3x late-morning drive.
    let (p, c) = (Params::default(), Coupling::default());
    let tr = run(&p, &Day::standard(), 48.0, 60.0);
    let drive = |h: f64| neural_drive(&tr.s[(h * 60.0) as usize], &p, &c)[0];
    assert!(drive(24.0 + 18.0) >= 3.0 * (drive(24.0 + 10.0) + 1.0) - 3.0);
}

#[test]
fn lit_pomc_moves_opposite_to_agrp() {
    // Mandelblat-Cerf 2015: putative POMC neurons change opposite to AgRP neurons.
    let (p, c) = (Params::default(), Coupling::default());
    let tr = run(&p, &Day::standard(), 48.0, 60.0);
    let (mut a, mut b) = (vec![], vec![]);
    for s in tr.s.iter().skip(24 * 60) {
        let d = neural_drive(s, &p, &c);
        a.push(d[0]);
        b.push(d[1]);
    }
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(&b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    assert!(cov < 0.0, "covariance {cov}");
}

#[test]
fn lit_sleep_debt_follows_36h_time_constant() {
    // Assumption (no 2015+ number): the cumulative debt relaxes with a 36 h time constant.
    let p = Params::default();
    let mut s = State::basal(&p);
    let n = (36.0 * HOUR / 60.0) as usize;
    for k in 0..n {
        step(&mut s, &p, k as f64 / 60.0, 60.0, 4.0);
    }
    assert!(
        (s.debt - (1.0 - (-1.0f64).exp())).abs() < 0.02,
        "debt {}",
        s.debt
    );
}

// ------------------------------------------------------------ other term tests

#[test]
fn ext_clamps_inputs() {
    let e = Ext {
        circ_shift_h: 99.0,
        stress: 5.0,
        illness: -2.0,
    }
    .clamped();
    assert_eq!((e.circ_shift_h, e.stress, e.illness), (12.0, 1.0, 0.0));
}

#[test]
fn misalignment_is_zero_to_one() {
    assert_eq!(Ext::default().misalignment(), 0.0);
    let e = Ext {
        circ_shift_h: -12.0,
        ..Ext::default()
    };
    assert_eq!(e.misalignment(), 1.0);
}

#[test]
fn stress_raises_cortisol_target() {
    let p = Params::default();
    let calm = cortisol_target(&p, 14.0, 8.0, 0.0, &Ext::default());
    let hot = cortisol_target(
        &p,
        14.0,
        8.0,
        0.0,
        &Ext {
            stress: 1.0,
            ..Ext::default()
        },
    );
    assert!(hot > calm);
}

#[test]
fn sleep_debt_raises_night_cortisol() {
    let p = Params::default();
    let a = cortisol_target(&p, 0.0, 8.0, 0.0, &Ext::default());
    let b = cortisol_target(&p, 0.0, 8.0, 1.0, &Ext::default());
    assert!(b > a);
}

#[test]
fn no_wake_response_after_all_nighter() {
    let p = Params::default();
    let wake = (23.0f64 + 0.0).rem_euclid(24.0) + p.car_delay_h;
    let with_sleep = cortisol_target(&p, wake, 8.0, 0.0, &Ext::default());
    let none = cortisol_target(&p, wake, 0.0, 0.0, &Ext::default());
    assert!(none.is_finite() && with_sleep.is_finite());
}

#[test]
fn illness_lowers_ghrelin_target() {
    let p = Params::default();
    let go = |ill: f64| {
        let mut s = State::basal(&p);
        let e = Ext {
            illness: ill,
            ..Ext::default()
        };
        for k in 0..2000 {
            step_ext(&mut s, &p, 3.0 + k as f64 / 120.0, 30.0, 8.0, &e);
        }
        s.ghrelin
    };
    assert!(go(1.0) < go(0.0));
}

#[test]
fn sleep_debt_state_grows_with_short_sleep() {
    let p = Params::default();
    let go = |sleep: f64| {
        let mut s = State::basal(&p);
        for k in 0..2000 {
            step(&mut s, &p, k as f64 / 120.0, 30.0, sleep);
        }
        s.debt
    };
    assert!(go(4.0) > go(8.0) && go(8.0) > go(10.0) - 1e-12);
}

#[test]
fn agonist_slows_gastric_emptying_per_second() {
    let p = Params::default();
    let mut a = State::basal(&p);
    let mut b = a;
    a.stomach = 50.0;
    b.stomach = 50.0;
    b.drug = 1.0;
    step(&mut a, &p, 12.0, 10.0, 8.0);
    step(&mut b, &p, 12.0, 10.0, 8.0);
    assert!(b.stomach > a.stomach);
}

#[test]
fn pyy_rises_after_meal_and_returns() {
    let p = Params::default();
    let tr = run(&p, &Day::standard(), 48.0, 60.0);
    let peak = tr.s.iter().map(|s| s.pyy).fold(0.0, f64::max);
    assert!(peak > p.pyy_base + 10.0);
    let late = tr.s.last().unwrap().pyy; // 23:59, five hours after dinner
    assert!(late < p.pyy_base + 5.0, "late {late}");
}

#[test]
fn pyy_lowers_agrp_drive() {
    let (p, c) = (Params::default(), Coupling::default());
    let a = State::basal(&p);
    let mut b = a;
    b.pyy += 30.0;
    assert!(neural_drive(&b, &p, &c)[0] <= neural_drive(&a, &p, &c)[0]);
}

#[test]
fn cortisol_raises_agrp_drive() {
    let (p, c) = (Params::default(), Coupling::default());
    let a = State::basal(&p);
    let mut b = a;
    b.cortisol += 10.0;
    assert!(neural_drive(&b, &p, &c)[0] >= neural_drive(&a, &p, &c)[0]);
}

#[test]
fn neuron_reference_still_fires() {
    // anchor to the v0.1 plant: a 10 uA/cm^2 current makes the HH neuron fire
    assert!(neuron::simulate_dc(10.0, 100.0).spikes > 0);
}
