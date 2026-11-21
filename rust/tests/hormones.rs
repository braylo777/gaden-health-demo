use gaden::fit;
use gaden::hormones::*;

fn std_trace(hours: f64) -> Trace {
    run(&Params::default(), &Day::standard(), hours, 60.0)
}

fn mean_between(tr: &Trace, h0: f64, h1: f64, f: impl Fn(&State) -> f64) -> f64 {
    let v: Vec<f64> = (0..tr.h.len())
        .filter(|&i| tr.h[i] >= h0 && tr.h[i] < h1)
        .map(|i| f(&tr.s[i]))
        .collect();
    v.iter().sum::<f64>() / v.len() as f64
}

#[test]
fn basal_state_is_finite() {
    assert!(State::basal(&Params::default()).is_finite());
}

#[test]
fn basal_state_uses_params() {
    let p = Params::default();
    let s = State::basal(&p);
    assert_eq!(s.leptin, p.lep_mean);
    assert_eq!(s.ghrelin, p.ghr_base);
    assert_eq!(s.glucose, p.glu_base);
}

#[test]
fn prior_differs_from_fitted_default() {
    assert_ne!(Params::prior(), Params::default());
}

#[test]
fn prior_keeps_unfitted_values() {
    let (a, b) = (Params::prior(), Params::default());
    assert_eq!(a.ins_gain, b.ins_gain);
    assert_eq!(a.drug_tau_s, b.drug_tau_s);
}

#[test]
fn trace_length_matches_hours() {
    assert_eq!(std_trace(24.0).h.len(), 1440);
}

#[test]
fn trace_stays_finite_for_two_days() {
    assert!(std_trace(48.0).s.iter().all(|s| s.is_finite()));
}

#[test]
fn all_hormones_stay_positive() {
    for s in std_trace(48.0).s {
        assert!(s.ghrelin > 0.0 && s.leptin > 0.0 && s.insulin > 0.0 && s.glp1 > 0.0);
    }
}

#[test]
fn gut_contents_never_negative() {
    for s in std_trace(48.0).s {
        assert!(s.stomach >= 0.0 && s.gut >= 0.0 && s.depot >= 0.0 && s.drug >= 0.0);
    }
}

#[test]
fn glucose_has_floor() {
    for s in std_trace(48.0).s {
        assert!(s.glucose >= 20.0);
    }
}

#[test]
fn eat_adds_carbs_to_stomach() {
    let mut s = State::basal(&Params::default());
    eat(
        &mut s,
        &Meal {
            at_h: 8.0,
            carbs_g: 50.0,
        },
    );
    assert_eq!(s.stomach, 50.0);
    assert_eq!(s.since_meal, 0.0);
}

#[test]
fn eat_ignores_negative_meal() {
    let mut s = State::basal(&Params::default());
    let before = s.since_meal;
    eat(
        &mut s,
        &Meal {
            at_h: 8.0,
            carbs_g: -5.0,
        },
    );
    assert_eq!(s.stomach, 0.0);
    assert_eq!(s.since_meal, before);
}

#[test]
fn inject_adds_to_depot() {
    let mut s = State::basal(&Params::default());
    inject(
        &mut s,
        &Dose {
            at_h: 8.0,
            units: 1.5,
        },
    );
    assert_eq!(s.depot, 1.5);
}

#[test]
fn inject_ignores_negative_dose() {
    let mut s = State::basal(&Params::default());
    inject(
        &mut s,
        &Dose {
            at_h: 8.0,
            units: -1.0,
        },
    );
    assert_eq!(s.depot, 0.0);
}

#[test]
fn events_fire_once_per_day() {
    let p = Params::default();
    let mut s = State::basal(&p);
    let day = Day::standard();
    let mut eaten = 0.0;
    for k in 0..(24 * 60) {
        let before = s.stomach;
        apply_events(&mut s, &day, k as f64 / 60.0, 60.0);
        eaten += s.stomach - before;
    }
    assert!((eaten - 225.0).abs() < 1e-9);
}

#[test]
fn events_repeat_on_day_two() {
    let p = Params::default();
    let mut s = State::basal(&p);
    apply_events(&mut s, &Day::standard(), 32.0, 60.0); // 08:00 on day 2
    assert_eq!(s.stomach, 60.0);
}

#[test]
fn meal_raises_glucose() {
    let tr = std_trace(48.0);
    let pre = tr.s[(32.0 * 60.0) as usize - 1].glucose;
    let post = mean_between(&tr, 32.5, 33.5, |s| s.glucose);
    assert!(post > pre + 5.0, "pre {pre} post {post}");
}

#[test]
fn meal_raises_insulin() {
    let tr = std_trace(48.0);
    let pre = tr.s[(32.0 * 60.0) as usize - 1].insulin;
    let post = mean_between(&tr, 32.5, 33.5, |s| s.insulin);
    assert!(post > pre * 1.5);
}

#[test]
fn meal_raises_glp1() {
    let tr = std_trace(48.0);
    let pre = tr.s[(32.0 * 60.0) as usize - 1].glp1;
    let post = mean_between(&tr, 32.25, 33.0, |s| s.glp1);
    assert!(post > pre * 1.5);
}

#[test]
fn meal_lowers_ghrelin() {
    let tr = std_trace(48.0);
    let pre = tr.s[(32.0 * 60.0) as usize].ghrelin;
    let post = mean_between(&tr, 32.75, 33.25, |s| s.ghrelin);
    assert!(post < pre);
}

#[test]
fn ghrelin_rises_between_meals() {
    let tr = std_trace(48.0);
    let early = tr.s[(34.0 * 60.0) as usize].ghrelin;
    let late = tr.s[(36.49 * 60.0) as usize].ghrelin;
    assert!(late > early);
}

#[test]
fn insulin_and_ghrelin_are_reciprocal() {
    // Cummings 2001: ghrelin pattern is reciprocal to insulin. Check the sign of correlation.
    let tr = std_trace(48.0);
    let d: Vec<&State> = (0..tr.h.len())
        .filter(|&i| tr.h[i] >= 24.0)
        .map(|i| &tr.s[i])
        .collect();
    let n = d.len() as f64;
    let (mi, mg) = (
        d.iter().map(|s| s.insulin).sum::<f64>() / n,
        d.iter().map(|s| s.ghrelin).sum::<f64>() / n,
    );
    let cov: f64 = d.iter().map(|s| (s.insulin - mi) * (s.ghrelin - mg)).sum();
    assert!(cov < 0.0);
}

#[test]
fn fasting_raises_ghrelin_above_fed() {
    let p = Params::default();
    let fast = run(
        &p,
        &Day {
            meals: vec![],
            ..Day::standard()
        },
        48.0,
        60.0,
    );
    let fed = std_trace(48.0);
    let m = |t: &Trace| mean_between(t, 24.0, 48.0, |s| s.ghrelin);
    assert!(m(&fast) > m(&fed));
}

#[test]
fn fasting_keeps_glucose_near_basal() {
    let p = Params::default();
    let tr = run(
        &p,
        &Day {
            meals: vec![],
            ..Day::standard()
        },
        48.0,
        60.0,
    );
    for s in &tr.s {
        assert!((s.glucose - p.glu_base).abs() < 1.0);
    }
}

#[test]
fn bigger_meal_bigger_glucose_peak() {
    let p = Params::default();
    let peak = |g: f64| {
        let t = run(&p, &fit::single_meal_day(g), 24.0, 60.0);
        t.s.iter().map(|s| s.glucose).fold(0.0, f64::max)
    };
    assert!(peak(120.0) > peak(60.0));
}

#[test]
fn leptin_shape_mean_is_one() {
    let p = Params::default();
    let m: f64 = (0..2400)
        .map(|i| leptin_shape(&p, i as f64 / 100.0))
        .sum::<f64>()
        / 2400.0;
    assert!((m - 1.0).abs() < 1e-9);
}

#[test]
fn leptin_shape_is_24h_periodic() {
    let p = Params::default();
    for h in [0.0, 3.3, 11.0, 17.7] {
        assert!((leptin_shape(&p, h) - leptin_shape(&p, h + 24.0)).abs() < 1e-12);
    }
}

#[test]
fn leptin_shape_positive() {
    let p = Params::default();
    for i in 0..240 {
        assert!(leptin_shape(&p, i as f64 / 10.0) > 0.0);
    }
}

#[test]
fn leptin_peaks_at_night() {
    let tr = std_trace(48.0);
    let night = mean_between(&tr, 24.0, 27.0, |s| s.leptin);
    let noon = mean_between(&tr, 34.0, 37.0, |s| s.leptin);
    assert!(night > noon);
}

#[test]
fn sleep_debt_bounds() {
    assert_eq!(sleep_debt(10.0), 0.0);
    assert_eq!(sleep_debt(12.0), 0.0);
    assert_eq!(sleep_debt(4.0), 1.0);
    assert_eq!(sleep_debt(0.0), 1.0);
    assert!((sleep_debt(7.0) - 0.5).abs() < 1e-12);
}

#[test]
fn sleep_debt_monotone() {
    let mut prev = 1.0;
    for i in 0..=120 {
        let d = sleep_debt(i as f64 / 10.0);
        assert!(d <= prev + 1e-12);
        prev = d;
    }
}

#[test]
fn sleep_loss_lowers_leptin() {
    let p = Params::default();
    let a = run(
        &p,
        &Day {
            sleep_h: 4.0,
            ..Day::standard()
        },
        48.0,
        60.0,
    );
    let b = std_trace(48.0);
    let m = |t: &Trace| mean_between(t, 24.0, 48.0, |s| s.leptin);
    assert!(m(&a) < m(&b));
}

#[test]
fn sleep_loss_raises_ghrelin() {
    let p = Params::default();
    let a = run(
        &p,
        &Day {
            sleep_h: 4.0,
            ..Day::standard()
        },
        48.0,
        60.0,
    );
    let b = std_trace(48.0);
    let m = |t: &Trace| mean_between(t, 24.0, 48.0, |s| s.ghrelin);
    assert!(m(&a) > m(&b));
}

#[test]
fn dose_raises_total_glp1() {
    let p = Params::default();
    let d = Day {
        doses: vec![Dose {
            at_h: 8.0,
            units: 1.0,
        }],
        ..Day::standard()
    };
    let a = run(&p, &d, 48.0, 60.0);
    let b = std_trace(48.0);
    let m = |t: &Trace| mean_between(t, 24.0, 48.0, |s| s.glp1_total(&p));
    assert!(m(&a) > m(&b) + 1.0);
}

#[test]
fn no_dose_means_no_drug() {
    assert!(std_trace(48.0).s.iter().all(|s| s.drug == 0.0));
}

#[test]
fn glp1_total_adds_drug() {
    let p = Params::default();
    let mut s = State::basal(&p);
    s.drug = 2.0;
    assert!((s.glp1_total(&p) - (p.glp1_base + 2.0 * p.drug_pmol_per_unit)).abs() < 1e-12);
}

#[test]
fn neural_drive_at_basal() {
    let p = Params::default();
    let c = Coupling::default();
    let d = neural_drive(&State::basal(&p), &p, &c);
    assert!((d[0] - c.agrp_rest).abs() < 1e-9);
    assert!((d[1] - c.pomc_rest).abs() < 1e-9);
}

#[test]
fn neural_drive_ghrelin_excites_agrp() {
    let p = Params::default();
    let c = Coupling::default();
    let mut s = State::basal(&p);
    s.ghrelin *= 1.2;
    assert!(neural_drive(&s, &p, &c)[0] > c.agrp_rest);
}

#[test]
fn neural_drive_leptin_inhibits_agrp_excites_pomc() {
    let p = Params::default();
    let c = Coupling::default();
    let mut s = State::basal(&p);
    s.leptin *= 1.3;
    let d = neural_drive(&s, &p, &c);
    assert!(d[0] < c.agrp_rest && d[1] > c.pomc_rest);
}

#[test]
fn neural_drive_is_clamped() {
    let p = Params::default();
    let c = Coupling::default();
    let mut s = State::basal(&p);
    s.ghrelin *= 100.0;
    assert_eq!(neural_drive(&s, &p, &c)[0], 120.0);
    s = State::basal(&p);
    s.leptin *= 100.0;
    assert_eq!(neural_drive(&s, &p, &c)[0], 0.0);
}

#[test]
fn step_is_deterministic() {
    let p = Params::default();
    let (mut a, mut b) = (State::basal(&p), State::basal(&p));
    for k in 0..500 {
        step(&mut a, &p, k as f64 / 60.0, 60.0, 8.0);
        step(&mut b, &p, k as f64 / 60.0, 60.0, 8.0);
    }
    assert_eq!(a, b);
}

#[test]
fn finer_step_agrees_with_coarse() {
    // Euler at 60 s vs 10 s: daily leptin mean within 1%.
    let p = Params::default();
    let a = run(&p, &Day::standard(), 48.0, 60.0);
    let b = run(&p, &Day::standard(), 48.0, 10.0);
    let m = |t: &Trace| mean_between(t, 24.0, 48.0, |s| s.leptin);
    assert!((m(&a) - m(&b)).abs() / m(&b) < 0.01);
}

#[test]
fn as_array_has_ten_fields() {
    assert_eq!(State::basal(&Params::default()).as_array().len(), 10);
}

#[test]
fn nan_is_detected() {
    let mut s = State::basal(&Params::default());
    s.insulin = f64::NAN;
    assert!(!s.is_finite());
}
