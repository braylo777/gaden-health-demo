//! Safety filter, robust MPC and bounded-output tests. The property tests use a seeded
//! generator (no extra crates) and run 10,000 random trials each.

use gaden::control::{Plant, Rng};
use gaden::controllers::*;

fn uni(r: &mut Rng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * r.next_f64()
}

/// Adversarial inner controller: returns random values, including NaN, inf, negative and huge.
struct Wild(Rng);
impl Controller for Wild {
    fn name(&self) -> &'static str {
        "wild"
    }
    fn step(&mut self, _y: [f64; 2], _dt: f64, _u: [f64; 2]) -> [f64; 2] {
        let mut out = [0.0; 2];
        for o in &mut out {
            let k = self.0.next_f64();
            *o = if k < 0.05 {
                f64::NAN
            } else if k < 0.10 {
                f64::INFINITY
            } else if k < 0.15 {
                f64::NEG_INFINITY
            } else if k < 0.35 {
                -1e6 * self.0.next_f64()
            } else if k < 0.55 {
                1e6 * self.0.next_f64()
            } else {
                100.0 * self.0.next_f64()
            };
        }
        out
    }
    fn reset(&mut self) {}
}

fn wild_safe(seed: u64) -> Safe {
    Safe::new(
        Box::new(Wild(Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1))),
        SafetyLimits::default(),
        "safe_wild",
    )
}

#[test]
fn prop_safety_limits_never_broken_10k() {
    let lim = SafetyLimits::default();
    let mut r = Rng(0x5AFE);
    for trial in 0..10_000u64 {
        let mut k = wild_safe(trial + 1);
        let u_max = [uni(&mut r, 5.0, 60.0), uni(&mut r, 5.0, 40.0)];
        let mut last = [0.0; 2];
        for _ in 0..60 {
            let dt = uni(&mut r, 0.05, 5.0);
            let u = k.step([0.0; 2], dt, u_max);
            for i in 0..2 {
                assert!(
                    u[i].is_finite() && u[i] >= 0.0,
                    "trial {trial}: u = {}",
                    u[i]
                );
                assert!(
                    u[i] <= u_max[i] + 1e-12,
                    "trial {trial}: above device limit"
                );
                assert!(u[i] <= lim.peak[i] + 1e-12, "trial {trial}: above peak");
                assert!(
                    u[i] - last[i] <= lim.slew[i] * dt + 1e-9,
                    "trial {trial}: rise {} > slew",
                    u[i] - last[i]
                );
                assert!(
                    k.mean()[i] <= lim.mean_limit[i] + 1e-9,
                    "trial {trial}: mean"
                );
                last[i] = u[i];
            }
        }
    }
}

#[test]
fn prop_safety_mean_budget_holds_over_long_saturation() {
    // Ask for the maximum for 12 h at 1 s steps. The running mean must stay at or below the limit.
    let lim = SafetyLimits::default();
    let mut k = Safe::new(Box::new(Wild(Rng(7))), lim.clone(), "safe_wild");
    for _ in 0..(12 * 3600) {
        let u = k.filter([1e9, 1e9], 1.0, [40.0, 25.0]);
        assert!(u[0] <= lim.peak[0] && u[1] <= lim.peak[1]);
        assert!(k.mean()[0] <= lim.mean_limit[0] + 1e-9);
        assert!(k.mean()[1] <= lim.mean_limit[1] + 1e-9);
    }
    // and the budget really binds: the mean has reached the limit
    assert!(
        k.mean()[0] > 0.97 * lim.mean_limit[0],
        "mean {}",
        k.mean()[0]
    );
}

#[test]
fn prop_every_controller_output_in_bounds_10k() {
    let plant = Plant::default();
    let mut r = Rng(0xB0D5);
    for trial in 0..10_000u64 {
        let name = ALL_NAMES[(trial % ALL_NAMES.len() as u64) as usize];
        let mut k = by_name(name, trial).unwrap();
        for _ in 0..20 {
            let y = [uni(&mut r, -5.0, 80.0), uni(&mut r, -5.0, 80.0)];
            let u = k.step(y, uni(&mut r, 0.1, 2.0), plant.u_max);
            for i in 0..2 {
                assert!(
                    u[i].is_finite() && u[i] >= 0.0 && u[i] <= plant.u_max[i] + 1e-12,
                    "{name}: u[{i}] = {}",
                    u[i]
                );
            }
        }
    }
}

#[test]
fn safe_wrappers_respect_peak_in_closed_loop() {
    let lim = SafetyLimits::default();
    for name in ["safe_pid", "safe_rmpc"] {
        let mut k = by_name(name, 1).unwrap();
        for _ in 0..3000 {
            let u = k.step([30.0, 0.0], 1.0, [40.0, 25.0]);
            assert!(
                u[0] <= lim.peak[0] + 1e-12 && u[1] <= lim.peak[1] + 1e-12,
                "{name}"
            );
        }
    }
}

#[test]
fn safe_filter_clamps_nan_to_zero() {
    let mut k = wild_safe(1);
    let u = k.filter([f64::NAN, f64::NAN], 1.0, [40.0, 25.0]);
    assert_eq!(u, [0.0, 0.0]);
}

#[test]
fn safe_filter_clamps_infinity_to_zero() {
    let mut k = wild_safe(1);
    let u = k.filter([f64::INFINITY, f64::NEG_INFINITY], 1.0, [40.0, 25.0]);
    assert_eq!(u, [0.0, 0.0]);
}

#[test]
fn safe_filter_negative_to_zero() {
    let mut k = wild_safe(1);
    assert_eq!(k.filter([-5.0, -1e9], 1.0, [40.0, 25.0]), [0.0, 0.0]);
}

#[test]
fn safe_filter_slew_limits_rise() {
    let mut k = wild_safe(1);
    let u1 = k.filter([30.0, 20.0], 1.0, [40.0, 25.0]);
    assert!((u1[0] - 10.0).abs() < 1e-12 && (u1[1] - 10.0).abs() < 1e-12);
    let u2 = k.filter([30.0, 20.0], 1.0, [40.0, 25.0]);
    assert!((u2[0] - 20.0).abs() < 1e-12);
}

#[test]
fn safe_filter_fall_is_not_rate_limited() {
    let mut k = wild_safe(1);
    for _ in 0..10 {
        k.filter([30.0, 20.0], 1.0, [40.0, 25.0]);
    }
    let u = k.filter([0.0, 0.0], 1.0, [40.0, 25.0]);
    assert_eq!(u, [0.0, 0.0]);
}

#[test]
fn safe_filter_peak_below_device_limit() {
    let mut k = wild_safe(1);
    for _ in 0..20 {
        k.filter([1e3, 1e3], 1.0, [40.0, 25.0]);
    }
    let u = k.filter([1e3, 1e3], 1.0, [40.0, 25.0]);
    assert!(
        (u[0] - 38.0).abs() < 1e-9 && (u[1] - 23.75).abs() < 1e-9,
        "{u:?}"
    );
}

#[test]
fn safe_filter_device_limit_below_peak_wins() {
    let mut k = wild_safe(1);
    for _ in 0..20 {
        k.filter([1e3, 1e3], 1.0, [10.0, 5.0]);
    }
    assert_eq!(k.filter([1e3, 1e3], 1.0, [10.0, 5.0]), [10.0, 5.0]);
}

#[test]
fn safe_filter_zero_dt_is_safe() {
    let mut k = wild_safe(1);
    let u = k.filter([30.0, 30.0], 0.0, [40.0, 25.0]);
    assert!(u.iter().all(|v| v.is_finite() && *v >= 0.0));
}

#[test]
fn safe_filter_negative_dt_is_safe() {
    let mut k = wild_safe(1);
    let u = k.filter([30.0, 30.0], -1.0, [40.0, 25.0]);
    assert!(u.iter().all(|v| v.is_finite() && *v >= 0.0));
}

#[test]
fn safe_reset_clears_state() {
    let mut k = by_name("safe_pid", 1).unwrap();
    for _ in 0..50 {
        k.step([30.0, 0.0], 1.0, [40.0, 25.0]);
    }
    k.reset();
    let u = k.step([30.0, 0.0], 1.0, [40.0, 25.0]);
    assert!(u[0] <= SafetyLimits::default().slew[0] + 1e-12);
}

#[test]
fn safe_names_are_set() {
    assert_eq!(by_name("safe_pid", 1).unwrap().name(), "safe_pid");
    assert_eq!(by_name("safe_rmpc", 1).unwrap().name(), "safe_rmpc");
    assert_eq!(by_name("rmpc", 1).unwrap().name(), "rmpc");
}

#[test]
fn all_names_resolve_and_are_unique() {
    let mut seen = vec![];
    for n in ALL_NAMES {
        let k = by_name(n, 1).unwrap();
        assert_eq!(k.name(), n);
        assert!(!seen.contains(&n));
        seen.push(n);
    }
    assert!(by_name("nope", 1).is_none());
}

#[test]
fn all_names_contains_the_v02_names() {
    for n in NAMES {
        assert!(ALL_NAMES.contains(&n));
    }
}

#[test]
fn safety_limits_are_inside_device_limits() {
    let (l, p) = (SafetyLimits::default(), Plant::default());
    for i in 0..2 {
        assert!(l.peak[i] <= p.u_max[i]);
        assert!(l.mean_limit[i] <= l.peak[i]);
        assert!(l.slew[i] > 0.0);
    }
}

// ------------------------------------------------------------ robust MPC

#[test]
fn rmpc_single_scale_matches_mpc() {
    let mut r = Rng(11);
    let m = Mpc::default();
    let rm = RobustMpc {
        scales: vec![1.0],
        ..RobustMpc::default()
    };
    for _ in 0..300 {
        let (x0, d) = (uni(&mut r, 0.0, 40.0), uni(&mut r, 0.0, 50.0));
        for i in 0..2 {
            let a = m.solve(i, x0, d, 1.0).clamp(0.0, 40.0);
            let b = rm.solve(i, x0, d, 1.0, 40.0);
            assert!((a - b).abs() < 1e-3, "ch {i}: {a} vs {b}");
        }
    }
}

#[test]
fn rmpc_output_inside_limits() {
    let rm = RobustMpc::default();
    let mut r = Rng(12);
    for _ in 0..500 {
        let u = rm.solve(0, uni(&mut r, 0.0, 60.0), uni(&mut r, 0.0, 80.0), 1.0, 40.0);
        assert!((0.0..=40.0).contains(&u));
    }
}

#[test]
fn rmpc_worst_case_cost_not_worse_than_nominal_mpc() {
    // Evaluate the worst-case horizon cost over gains {0.7, 1.0, 1.3} at both inputs.
    let (m, rm) = (Mpc::default(), RobustMpc::default());
    let cost = |i: usize, x0: f64, d: f64, u: f64, k: f64| {
        let a = 1.0 - 1.0 / m.tau;
        let (mut ak, mut c) = (1.0, 0.0);
        for _ in 0..m.horizon {
            ak *= a;
            let b = m.gain[i] * k * (1.0 - ak);
            let e = d * (1.0 - ak) + x0 * ak - m.target[i] + b * u;
            c += e * e;
        }
        c + m.lambda[i] * u * u
    };
    let worst = |i, x0, d, u| {
        [0.7, 1.0, 1.3]
            .iter()
            .map(|&k| cost(i, x0, d, u, k))
            .fold(f64::MIN, f64::max)
    };
    let mut r = Rng(13);
    for _ in 0..400 {
        let (x0, d) = (uni(&mut r, 0.0, 40.0), uni(&mut r, 0.0, 60.0));
        let un = m.solve(0, x0, d, 1.0).clamp(0.0, 40.0);
        let ur = rm.solve(0, x0, d, 1.0, 40.0);
        assert!(
            worst(0, x0, d, ur) <= worst(0, x0, d, un) + 1e-6,
            "x0 {x0} d {d}"
        );
    }
}

#[test]
fn rmpc_acts_harder_than_mpc_when_gain_may_be_low() {
    // With a possible 30% gain loss, the robust input is at least the nominal one for AgRP.
    let (m, rm) = (Mpc::default(), RobustMpc::default());
    let un = m.solve(0, 20.0, 25.0, 1.0).clamp(0.0, 40.0);
    let ur = rm.solve(0, 20.0, 25.0, 1.0, 40.0);
    assert!(ur >= un - 1e-6, "{ur} vs {un}");
}

#[test]
fn rmpc_zero_error_gives_zero_input() {
    let rm = RobustMpc::default();
    // already at target and no disturbance beyond holding it: x0 = d = target of AgRP
    assert!(rm.solve(0, 1.0, 1.0, 1.0, 40.0) < 1.0);
}

#[test]
fn rmpc_reset_clears_disturbance() {
    let mut k = RobustMpc::default();
    k.step([30.0, 3.0], 1.0, [40.0, 25.0]);
    k.step([29.0, 3.0], 1.0, [40.0, 25.0]);
    k.reset();
    let a = k.step([30.0, 3.0], 1.0, [40.0, 25.0]);
    let mut f = RobustMpc::default();
    let b = f.step([30.0, 3.0], 1.0, [40.0, 25.0]);
    assert_eq!(a, b);
}

#[test]
fn rmpc_deterministic() {
    let run = || {
        let mut k = RobustMpc::default();
        (0..200)
            .map(|i| k.step([20.0 - 0.05 * i as f64, 7.0], 1.0, [40.0, 25.0])[0])
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
