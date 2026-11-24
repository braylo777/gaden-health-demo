//! The 1,000-scenario stress suite: generator, pass rule, confidence interval and a run of
//! every controller on every scenario kind.

use gaden::controllers::{self, ALL_NAMES};
use gaden::hormones::{Coupling, Params};
use gaden::sim::{self, wilson_ci, STRESS_KINDS};

fn per_kind(kind: &str) -> Vec<sim::Scenario> {
    sim::suite_1000()
        .into_iter()
        .filter(|s| s.category == kind)
        .collect()
}

#[test]
fn suite_has_1000_scenarios() {
    assert_eq!(sim::suite_1000().len(), 1000);
}

#[test]
fn suite_is_deterministic() {
    assert_eq!(sim::suite_1000(), sim::suite_1000());
}

#[test]
fn suite_changes_with_seed() {
    assert_ne!(sim::stress_suite(50, 1), sim::stress_suite(50, 2));
}

#[test]
fn suite_prefix_is_stable() {
    assert_eq!(
        sim::stress_suite(100, sim::STRESS_SEED),
        sim::suite_1000()[..100]
    );
}

#[test]
fn suite_has_200_per_kind() {
    for k in STRESS_KINDS {
        assert_eq!(per_kind(k).len(), 200, "{k}");
    }
}

#[test]
fn names_and_seeds_are_unique() {
    let s = sim::suite_1000();
    let mut names: Vec<_> = s.iter().map(|x| x.name.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 1000);
    let mut seeds: Vec<_> = s.iter().map(|x| x.seed).collect();
    seeds.sort();
    seeds.dedup();
    assert_eq!(seeds.len(), 1000);
}

#[test]
fn gain_errors_stay_in_30_percent_band() {
    for s in sim::suite_1000() {
        for g in s.gain_err {
            assert!((0.7..=1.3).contains(&g), "{} gain {g}", s.name);
        }
    }
}

#[test]
fn sensor_noise_scenarios_span_half_to_two_hz() {
    let v = per_kind("sensor_noise");
    assert!(v.iter().all(|s| (0.5..=2.0 + 1e-9).contains(&s.noise)));
    let (lo, hi) = v.iter().fold((9.0f64, 0.0f64), |a, s| {
        (a.0.min(s.noise), a.1.max(s.noise))
    });
    assert!(lo < 0.7 && hi > 1.5, "range {lo}..{hi}");
}

#[test]
fn other_kinds_use_default_noise() {
    for k in ["shift_work", "illness", "travel", "missed_meals"] {
        assert!(
            per_kind(k).iter().all(|s| (s.noise - 0.3).abs() < 1e-12),
            "{k}"
        );
    }
}

#[test]
fn shift_work_moves_sleep_and_clock() {
    for s in per_kind("shift_work") {
        assert!((4.0..=7.0).contains(&s.day.sleep_h), "{}", s.name);
        assert!(s.ext.circ_shift_h.abs() >= 3.0, "{}", s.name);
    }
}

#[test]
fn travel_shifts_clock_three_to_nine_hours() {
    let v = per_kind("travel");
    assert!(v
        .iter()
        .all(|s| (3.0..=9.0).contains(&s.ext.circ_shift_h.abs())));
    assert!(
        v.iter().any(|s| s.ext.circ_shift_h > 0.0) && v.iter().any(|s| s.ext.circ_shift_h < 0.0)
    );
}

#[test]
fn illness_scenarios_have_illness_and_stress() {
    for s in per_kind("illness") {
        assert!(s.ext.illness >= 0.3 && s.ext.stress >= 0.2, "{}", s.name);
    }
}

#[test]
fn missed_meal_scenarios_have_fewer_meals() {
    for s in per_kind("missed_meals") {
        assert!(s.day.meals.len() < 3, "{}", s.name);
    }
}

#[test]
fn some_missed_meal_scenarios_are_full_fasts() {
    assert!(per_kind("missed_meals")
        .iter()
        .any(|s| s.day.meals.is_empty()));
}

#[test]
fn some_scenarios_include_a_dose() {
    let n = sim::suite_1000()
        .iter()
        .filter(|s| !s.day.doses.is_empty())
        .count();
    assert!((150..=450).contains(&n), "{n} dosed");
}

#[test]
fn warm_up_and_day_share_meals() {
    for s in sim::suite_1000() {
        assert_eq!(s.warm.meals, s.day.meals);
    }
}

#[test]
fn legacy_suite_is_still_240() {
    assert_eq!(sim::suite().len(), 240);
}

#[test]
fn wilson_ci_zero_successes() {
    let (lo, hi) = wilson_ci(0, 1000);
    assert!(lo.abs() < 1e-12);
    assert!(hi > 0.0 && hi < 0.01);
}

#[test]
fn wilson_ci_all_successes() {
    let (lo, hi) = wilson_ci(1000, 1000);
    assert!((hi - 1.0).abs() < 1e-12);
    assert!(lo > 0.99);
}

#[test]
fn wilson_ci_half_known_value() {
    let (lo, hi) = wilson_ci(50, 100);
    assert!(
        (lo - 0.4038).abs() < 1e-3 && (hi - 0.5962).abs() < 1e-3,
        "{lo} {hi}"
    );
}

#[test]
fn wilson_ci_contains_estimate() {
    for k in (0..=1000).step_by(37) {
        let (lo, hi) = wilson_ci(k, 1000);
        let p = k as f64 / 1000.0;
        assert!(lo <= p + 1e-12 && p <= hi + 1e-12, "k {k}");
    }
}

#[test]
fn wilson_ci_narrows_with_n() {
    let w = |n| {
        let (a, b) = wilson_ci(n / 2, n);
        b - a
    };
    assert!(w(100) > w(1000) && w(1000) > w(10_000));
}

#[test]
fn wilson_ci_empty_sample() {
    assert_eq!(wilson_ci(0, 0), (0.0, 1.0));
}

#[test]
fn parallel_runner_matches_sequential() {
    let (p, c) = (Params::default(), Coupling::default());
    let sub = &sim::stress_suite(10, 99);
    let par = sim::run_suite_par("pid", sub, &p, &c, 4);
    let seq = sim::run_suite("pid", sub, &p, &c);
    assert_eq!(par.iter().filter(|o| o.passed).count(), seq.passed);
}

#[test]
fn parallel_runner_keeps_order() {
    let (p, c) = (Params::default(), Coupling::default());
    let sub = sim::stress_suite(10, 5);
    let a = sim::run_suite_par("pid", &sub, &p, &c, 1);
    let b = sim::run_suite_par("pid", &sub, &p, &c, 3);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.passed, y.passed);
        assert_eq!(x.result, y.result);
    }
}

#[test]
fn open_loop_fails_most_of_the_suite() {
    let (p, c) = (Params::default(), Coupling::default());
    let sub = sim::stress_suite(40, 7);
    let o = sim::run_suite_par("open", &sub, &p, &c, 4);
    assert!(o.iter().filter(|x| x.passed).count() <= 2);
}

fn kind_controller(kind: &str, name: &str) {
    let (p, c) = (Params::default(), Coupling::default());
    let sub: Vec<_> = per_kind(kind).into_iter().take(3).collect();
    for sc in &sub {
        let mut k = controllers::by_name(name, sc.seed).unwrap();
        let r = sim::run_day(sc, Some(k.as_mut()), &p, &c);
        assert!(r.finite && r.bounded, "{name} on {}", sc.name);
        let lim = controllers::SafetyLimits::default();
        if name.starts_with("safe_") {
            assert!(r.u_peak[0] <= lim.peak[0] + 1e-9 && r.u_peak[1] <= lim.peak[1] + 1e-9);
            assert!(r.u_mean[0] <= lim.mean_limit[0] + 1e-9);
        }
    }
}

macro_rules! kind_tests {
    ($($name:ident: ($k:expr, $c:expr),)*) => { $( #[test] fn $name() { kind_controller($k, $c); } )* };
}

kind_tests! {
    shift_work_pid: ("shift_work", "pid"),
    shift_work_mpc: ("shift_work", "mpc"),
    shift_work_bandit: ("shift_work", "bandit"),
    shift_work_rmpc: ("shift_work", "rmpc"),
    shift_work_safe_pid: ("shift_work", "safe_pid"),
    shift_work_safe_rmpc: ("shift_work", "safe_rmpc"),
    illness_pid: ("illness", "pid"),
    illness_mpc: ("illness", "mpc"),
    illness_bandit: ("illness", "bandit"),
    illness_rmpc: ("illness", "rmpc"),
    illness_safe_pid: ("illness", "safe_pid"),
    illness_safe_rmpc: ("illness", "safe_rmpc"),
    travel_pid: ("travel", "pid"),
    travel_mpc: ("travel", "mpc"),
    travel_bandit: ("travel", "bandit"),
    travel_rmpc: ("travel", "rmpc"),
    travel_safe_pid: ("travel", "safe_pid"),
    travel_safe_rmpc: ("travel", "safe_rmpc"),
    missed_meals_pid: ("missed_meals", "pid"),
    missed_meals_mpc: ("missed_meals", "mpc"),
    missed_meals_bandit: ("missed_meals", "bandit"),
    missed_meals_rmpc: ("missed_meals", "rmpc"),
    missed_meals_safe_pid: ("missed_meals", "safe_pid"),
    missed_meals_safe_rmpc: ("missed_meals", "safe_rmpc"),
    sensor_noise_pid: ("sensor_noise", "pid"),
    sensor_noise_mpc: ("sensor_noise", "mpc"),
    sensor_noise_bandit: ("sensor_noise", "bandit"),
    sensor_noise_rmpc: ("sensor_noise", "rmpc"),
    sensor_noise_safe_pid: ("sensor_noise", "safe_pid"),
    sensor_noise_safe_rmpc: ("sensor_noise", "safe_rmpc"),
}

#[test]
fn all_names_cover_the_kind_tests() {
    assert_eq!(ALL_NAMES.len(), 6);
    assert_eq!(STRESS_KINDS.len(), 5);
}

#[test]
fn run_day_is_deterministic_per_scenario() {
    let (p, c) = (Params::default(), Coupling::default());
    let sc = &sim::suite_1000()[3];
    let run = || {
        let mut k = controllers::by_name("pid", sc.seed).unwrap();
        sim::run_day(sc, Some(k.as_mut()), &p, &c)
    };
    assert_eq!(run(), run());
}

#[test]
fn anchors_agree_with_the_literature_windows() {
    let p = Params::default();
    let g = gaden::anchors::first_hour_emptying_ratio(&p);
    assert!((0.61..=0.87).contains(&g), "gastric {g}");
    let m = gaden::anchors::misalign_glucose_ratio(&p);
    assert!((1.01..=1.08).contains(&m), "misalign {m}");
    let c = gaden::anchors::car_peak_minutes(&p);
    assert!((28.0..=47.0).contains(&c), "car {c}");
}

#[test]
fn recorder_does_not_change_the_result() {
    let (p, c) = (Params::default(), Coupling::default());
    let sc = &sim::suite_1000()[8];
    let mut a = controllers::by_name("pid", sc.seed).unwrap();
    let mut b = controllers::by_name("pid", sc.seed).unwrap();
    let plain = sim::run_day(sc, Some(a.as_mut()), &p, &c);
    let (with, se) = sim::run_day_series(sc, Some(b.as_mut()), &p, &c, 60);
    assert_eq!(plain, with);
    assert_eq!(se.t_h.len(), 1440);
    assert_eq!(se.agrp.len(), 1440);
}

#[test]
fn series_channels_have_equal_length_and_are_finite() {
    let (p, c) = (Params::default(), Coupling::default());
    let (_, se) = sim::run_day_series(&sim::nominal(), None, &p, &c, 600);
    let n = se.t_h.len();
    for v in [
        &se.agrp,
        &se.pomc,
        &se.ghrelin,
        &se.leptin,
        &se.insulin,
        &se.glp1,
        &se.pyy,
        &se.cortisol,
        &se.glucose,
        &se.u_agrp,
        &se.u_pomc,
        &se.drive_agrp,
    ] {
        assert_eq!(v.len(), n);
        assert!(v.iter().all(|x| x.is_finite()));
    }
}

#[test]
fn series_stride_one_gives_a_sample_per_second() {
    let (p, c) = (Params::default(), Coupling::default());
    let (_, se) = sim::run_day_series(&sim::nominal(), None, &p, &c, 1);
    assert_eq!(se.t_h.len(), 86_400);
}
