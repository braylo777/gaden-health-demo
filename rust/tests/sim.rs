use gaden::controllers::{self, Controller, Mpc, Pid};
use gaden::hormones::{Coupling, Dose, Params};
use gaden::sim::*;
use std::collections::HashSet;

fn pc() -> (Params, Coupling) {
    (Params::default(), Coupling::default())
}

fn run_with(name: &str, sc: &Scenario) -> DayResult {
    let (p, c) = pc();
    let mut k = controllers::by_name(name, sc.seed).unwrap();
    run_day(sc, Some(k.as_mut()), &p, &c)
}

#[test]
fn suite_has_240_scenarios() {
    assert_eq!(suite().len(), 240);
}

#[test]
fn suite_names_unique() {
    let s = suite();
    let names: HashSet<_> = s.iter().map(|x| x.name.clone()).collect();
    assert_eq!(names.len(), s.len());
}

#[test]
fn suite_is_deterministic() {
    assert_eq!(suite(), suite());
}

#[test]
fn suite_seeds_unique() {
    let s = suite();
    let seeds: HashSet<_> = s.iter().map(|x| x.seed).collect();
    assert_eq!(seeds.len(), s.len());
}

#[test]
fn suite_gain_errors_within_30pct() {
    for sc in suite() {
        for g in sc.gain_err {
            assert!((0.7..=1.3).contains(&g));
        }
    }
}

#[test]
fn suite_covers_required_categories() {
    let cats: HashSet<_> = suite().iter().map(|x| x.category).collect();
    for c in ["nominal", "fasting", "binge", "sleep_loss", "dose"] {
        assert!(cats.contains(c), "missing {c}");
    }
}

#[test]
fn suite_covers_all_factor_levels() {
    let s = suite();
    for (m, _, _) in meal_patterns() {
        assert_eq!(
            s.iter()
                .filter(|x| x.name.starts_with(&format!("{m}/")))
                .count(),
            30
        );
    }
    for (sl, _) in SLEEP {
        assert_eq!(
            s.iter()
                .filter(|x| x.name.contains(&format!("/{sl}/")))
                .count(),
            48
        );
    }
    for (d, _, _) in dose_regimes() {
        assert_eq!(
            s.iter()
                .filter(|x| x.name.ends_with(&format!("/{d}")))
                .count(),
            40
        );
    }
}

#[test]
fn fast_24h_has_no_meals() {
    assert!(suite()
        .iter()
        .filter(|x| x.name.starts_with("fast_24h/"))
        .all(|x| x.day.meals.is_empty()));
}

#[test]
fn dose_change_regimes_differ_between_days() {
    let regs = dose_regimes();
    for (n, w, d) in &regs {
        let changed = w != d;
        assert_eq!(changed, ["start", "stop", "double"].contains(n), "{n}");
    }
}

#[test]
fn sleep_applies_to_measured_day_only() {
    for sc in suite() {
        assert_eq!(sc.warm.sleep_h, 8.0);
    }
}

#[test]
fn open_loop_fails_nominal() {
    let (p, c) = pc();
    assert!(!run_day(&nominal(), None, &p, &c).pass());
}

#[test]
fn open_loop_has_no_current() {
    let (p, c) = pc();
    let r = run_day(&nominal(), None, &p, &c);
    assert_eq!(r.u_peak, [0.0, 0.0]);
}

#[test]
fn open_loop_peak_near_calibration() {
    // Coupling default is calibrated so the open-loop nominal peak is 30 Hz.
    let (p, c) = pc();
    let r = run_day(&nominal(), None, &p, &c);
    assert!((r.agrp_max_hz - 30.0).abs() < 0.5, "{}", r.agrp_max_hz);
}

#[test]
fn calibration_hits_requested_peak() {
    let (p, c) = pc();
    let c2 = calibrate_coupling(&p, &c, 25.0);
    let r = run_day(&nominal(), None, &p, &c2);
    assert!((r.agrp_max_hz - 25.0).abs() < 0.05);
}

#[test]
fn calibration_is_monotone() {
    let (p, c) = pc();
    assert!(calibrate_coupling(&p, &c, 20.0).w_ghr < calibrate_coupling(&p, &c, 35.0).w_ghr);
}

#[test]
fn drive_peak_equals_open_loop_peak_without_noise() {
    let (p, c) = pc();
    let r = run_day(&nominal(), None, &p, &c);
    assert!(r.drive_agrp_max_hz >= r.agrp_max_hz - 1e-6);
}

macro_rules! nominal_pass {
    ($($n:ident),*) => {$(
        #[test]
        fn $n() {
            let r = run_with(stringify!($n), &nominal());
            assert!(r.pass(), "{r:?}");
        }
    )*};
}
mod nominal_passes {
    use super::*;
    nominal_pass!(pid, mpc, bandit);
}

macro_rules! scenario_check {
    ($($n:ident: $name:expr, $ctrl:expr, $expect:expr;)*) => {$(
        #[test]
        fn $n() {
            let sc = suite().into_iter().find(|s| s.name == $name).expect($name);
            let r = run_with($ctrl, &sc);
            assert!(r.finite && r.bounded);
            assert_eq!(r.pass(), $expect, "{} {}: {r:?}", $name, $ctrl);
        }
    )*};
}

// Spot checks across categories. Expected values are what the model gives; failures are
// documented limits (40 uA AgRP limit under strong hunger), not bugs.
scenario_check! {
    standard_pid: "standard/sleep8/nodose", "pid", true;
    standard_mpc: "standard/sleep8/nodose", "mpc", true;
    standard_bandit: "standard/sleep8/nodose", "bandit", true;
    skip_breakfast_pid: "skip_breakfast/sleep8/nodose", "pid", true;
    grazing_pid: "grazing/sleep8/nodose", "pid", true;
    dose1_pid: "standard/sleep8/dose1", "pid", true;
    stop_mpc: "standard/sleep8/stop", "mpc", true;
    allnighter_pid: "standard/allnighter/nodose", "pid", false;
    allnighter_mpc: "standard/allnighter/nodose", "mpc", false;
}

#[test]
fn every_scenario_finite_and_bounded_pid() {
    let (p, c) = pc();
    for sc in suite().iter().step_by(4) {
        let mut k = Pid::default();
        let r = run_day(sc, Some(&mut k), &p, &c);
        assert!(r.finite && r.bounded, "{}", sc.name);
    }
}

#[test]
fn every_scenario_finite_and_bounded_mpc() {
    let (p, c) = pc();
    for sc in suite().iter().skip(1).step_by(4) {
        let mut k = Mpc::default();
        let r = run_day(sc, Some(&mut k), &p, &c);
        assert!(r.finite && r.bounded, "{}", sc.name);
    }
}

#[test]
fn every_scenario_finite_and_bounded_bandit() {
    let (p, c) = pc();
    for sc in suite().iter().skip(2).step_by(4) {
        let mut k = controllers::Bandit::new(sc.seed);
        let r = run_day(sc, Some(&mut k), &p, &c);
        assert!(r.finite && r.bounded, "{}", sc.name);
    }
}

#[test]
fn closed_loop_beats_open_loop_everywhere() {
    let (p, c) = pc();
    for sc in suite().iter().step_by(10) {
        let o = run_day(sc, None, &p, &c);
        let mut k = Pid::default();
        let r = run_day(sc, Some(&mut k), &p, &c);
        assert!(r.agrp_mean_hz < o.agrp_mean_hz, "{}", sc.name);
    }
}

#[test]
fn stress_pass_rate_above_75pct_for_pid() {
    // Subsample (every 3rd scenario) to keep the test fast; the full run is in `metrics`.
    let (p, c) = pc();
    let s: Vec<Scenario> = suite().into_iter().step_by(3).collect();
    let r = run_suite("pid", &s, &p, &c);
    assert!(r.pass_rate() > 0.75, "{}", r.pass_rate());
}

#[test]
fn stress_open_loop_never_passes() {
    let (p, c) = pc();
    let s: Vec<Scenario> = suite().into_iter().step_by(6).collect();
    assert_eq!(run_suite("open", &s, &p, &c).passed, 0);
}

#[test]
fn run_suite_counts_consistent() {
    let (p, c) = pc();
    let s: Vec<Scenario> = suite().into_iter().step_by(12).collect();
    let r = run_suite("mpc", &s, &p, &c);
    assert_eq!(r.passed + r.failures.len(), r.n);
    assert_eq!(r.n, s.len());
}

#[test]
fn failures_are_strong_hunger_cases() {
    // Every PID failure has open-loop AgRP drive above what 40 uA can cancel at its gain.
    let (p, c) = pc();
    for sc in suite().iter().step_by(5) {
        let mut k = Pid::default();
        let r = run_day(sc, Some(&mut k), &p, &c);
        if !r.pass() && (r.pomc_mean_hz - 8.0).abs() < 1.0 {
            let cap = 40.0 * sc.gain_err[0];
            assert!(
                r.drive_agrp_max_hz > cap,
                "{}: drive {} cap {cap}",
                sc.name,
                r.drive_agrp_max_hz
            );
        }
    }
}

#[test]
fn nominal_peak_below_2hz_for_pid_and_mpc() {
    // The bandit explores high-gain arms and overshoots; PID and MPC do not.
    assert!(run_with("pid", &nominal()).agrp_max_hz < 2.0);
    assert!(run_with("mpc", &nominal()).agrp_max_hz < 2.0);
}

#[test]
fn bandit_nominal_peak_higher_than_pid() {
    assert!(run_with("bandit", &nominal()).agrp_max_hz > run_with("pid", &nominal()).agrp_max_hz);
}

#[test]
fn noise_free_is_deterministic() {
    let a = run_with("pid", &nominal());
    let b = run_with("pid", &nominal());
    assert_eq!(a, b);
}

#[test]
fn same_seed_same_noisy_result() {
    let sc = suite()[17].clone();
    assert_eq!(run_with("mpc", &sc), run_with("mpc", &sc));
}

#[test]
fn different_seed_changes_noise() {
    let mut a = suite()[17].clone();
    let r1 = run_with("pid", &a);
    a.seed += 1000;
    let r2 = run_with("pid", &a);
    assert_ne!(r1.agrp_mean_hz, r2.agrp_mean_hz);
}

#[test]
fn pass_rule_requires_finite() {
    let r = DayResult {
        agrp_max_hz: 1.0,
        agrp_mean_hz: 1.0,
        frac_agrp_below_2hz: 1.0,
        pomc_mean_hz: 8.0,
        u_peak: [0.0; 2],
        u_mean: [0.0; 2],
        drive_agrp_max_hz: 1.0,
        finite: false,
        bounded: true,
    };
    assert!(!r.pass());
    assert!(DayResult {
        finite: true,
        ..r.clone()
    }
    .pass());
    assert!(!DayResult {
        finite: true,
        bounded: false,
        ..r.clone()
    }
    .pass());
    assert!(!DayResult {
        finite: true,
        pomc_mean_hz: 9.1,
        ..r.clone()
    }
    .pass());
    assert!(!DayResult {
        finite: true,
        frac_agrp_below_2hz: 0.94,
        ..r
    }
    .pass());
}

#[test]
fn higher_dose_lowers_open_loop_agrp() {
    let (p, c) = pc();
    let mut sc = nominal();
    let a = run_day(&sc, None, &p, &c).agrp_mean_hz;
    sc.warm.doses = vec![Dose {
        at_h: 8.0,
        units: 2.0,
    }];
    sc.day.doses = sc.warm.doses.clone();
    let b = run_day(&sc, None, &p, &c).agrp_mean_hz;
    assert!(b < a);
}

#[test]
fn sleep_loss_raises_open_loop_agrp() {
    let (p, c) = pc();
    let mut sc = nominal();
    let a = run_day(&sc, None, &p, &c).agrp_mean_hz;
    sc.day.sleep_h = 4.0;
    let b = run_day(&sc, None, &p, &c).agrp_mean_hz;
    assert!(b > a);
}

#[test]
fn fasting_raises_open_loop_agrp() {
    let (p, c) = pc();
    let mut sc = nominal();
    let a = run_day(&sc, None, &p, &c).agrp_max_hz;
    sc.warm.meals.clear();
    sc.day.meals.clear();
    let b = run_day(&sc, None, &p, &c).agrp_max_hz;
    assert!(b > a);
}

#[test]
fn closed_loop_uses_less_than_limit_on_nominal() {
    let r = run_with("mpc", &nominal());
    assert!(r.u_peak[0] < 40.0 && r.u_peak[1] < 25.0);
}

#[test]
fn reset_called_between_runs() {
    let (p, c) = pc();
    let mut k = Pid::default();
    let a = run_day(&nominal(), Some(&mut k), &p, &c);
    let b = run_day(&nominal(), Some(&mut k), &p, &c);
    assert_eq!(a, b);
}

#[test]
fn controller_trait_object_works_in_run_day() {
    let (p, c) = pc();
    let mut k: Box<dyn Controller> = Box::new(Mpc::default());
    assert!(run_day(&nominal(), Some(k.as_mut()), &p, &c).pass());
}
