use gaden::control::{Plant, Rng};
use gaden::controllers::*;

const UMAX: [f64; 2] = [40.0, 25.0];

/// Run a controller on the nominal first-order plant with constant drive `d` for `secs`.
fn hold(c: &mut dyn Controller, d: [f64; 2], gain_err: [f64; 2], secs: usize) -> [f64; 2] {
    let p = Plant::default();
    let g = [p.gain[0] * gain_err[0], p.gain[1] * gain_err[1]];
    let mut x = d;
    for _ in 0..secs {
        let u = c.step(x, 1.0, UMAX);
        for i in 0..2 {
            x[i] = (x[i] + (d[i] + g[i] * u[i] - x[i]) / p.tau).max(0.0);
        }
    }
    x
}

macro_rules! each_controller {
    ($($name:ident: $body:expr;)*) => {
        $(
            mod $name {
                use super::*;
                #[test] fn pid() { ($body)(Box::new(Pid::default()) as Box<dyn Controller>) }
                #[test] fn mpc() { ($body)(Box::new(Mpc::default()) as Box<dyn Controller>) }
                #[test] fn bandit() { ($body)(Box::new(Bandit::new(7)) as Box<dyn Controller>) }
            }
        )*
    };
}

each_controller! {
    output_within_limits: |mut c: Box<dyn Controller>| {
        let mut r = Rng(99);
        for _ in 0..5000 {
            let y = [60.0 * r.next_f64(), 20.0 * r.next_f64()];
            let u = c.step(y, 1.0, UMAX);
            assert!(u[0] >= 0.0 && u[0] <= UMAX[0] && u[1] >= 0.0 && u[1] <= UMAX[1]);
        }
    };
    output_finite: |mut c: Box<dyn Controller>| {
        let mut r = Rng(5);
        for _ in 0..2000 {
            let y = [200.0 * r.next_f64(), 200.0 * r.next_f64()];
            let u = c.step(y, 1.0, UMAX);
            assert!(u[0].is_finite() && u[1].is_finite());
        }
    };
    regulates_nominal_drive: |mut c: Box<dyn Controller>| {
        let x = hold(c.as_mut(), [25.0, 3.0], [1.0, 1.0], 1200);
        assert!(x[0] < 2.0, "agrp {}", x[0]);
        assert!((x[1] - 8.0).abs() < 1.0, "pomc {}", x[1]);
    };
    regulates_with_low_gain: |mut c: Box<dyn Controller>| {
        let x = hold(c.as_mut(), [20.0, 3.0], [0.75, 0.75], 1800);
        assert!(x[0] < 2.0, "agrp {}", x[0]);
        assert!((x[1] - 8.0).abs() < 1.0, "pomc {}", x[1]);
    };
    regulates_with_high_gain: |mut c: Box<dyn Controller>| {
        let x = hold(c.as_mut(), [25.0, 3.0], [1.3, 1.3], 1800);
        assert!(x[0] < 2.0);
        assert!((x[1] - 8.0).abs() < 1.0);
    };
    saturates_when_drive_too_high: |mut c: Box<dyn Controller>| {
        // 60 Hz drive needs more than 40 uA: AgRP cannot reach target
        let x = hold(c.as_mut(), [60.0, 3.0], [1.0, 1.0], 1200);
        assert!(x[0] > 15.0);
    };
    zero_output_when_agrp_below_target_and_pomc_high: |mut c: Box<dyn Controller>| {
        let mut u = [1.0; 2];
        for _ in 0..600 {
            u = c.step([0.0, 20.0], 1.0, UMAX);
        }
        assert_eq!(u, [0.0, 0.0]);
    };
    reset_restores_initial_response: |mut c: Box<dyn Controller>| {
        let u0 = c.step([10.0, 5.0], 1.0, UMAX);
        for _ in 0..300 { c.step([30.0, 2.0], 1.0, UMAX); }
        c.reset();
        let u1 = c.step([10.0, 5.0], 1.0, UMAX);
        assert_eq!(u0, u1);
    };
    has_a_name: |c: Box<dyn Controller>| {
        assert!(NAMES.contains(&c.name()));
    };
    respects_smaller_limits: |mut c: Box<dyn Controller>| {
        for _ in 0..200 {
            let u = c.step([50.0, 0.0], 1.0, [5.0, 3.0]);
            assert!(u[0] <= 5.0 && u[1] <= 3.0);
        }
    };
}

#[test]
fn by_name_known() {
    for n in NAMES {
        assert_eq!(by_name(n, 1).unwrap().name(), n);
    }
}

#[test]
fn by_name_unknown() {
    assert!(by_name("lqr", 1).is_none());
}

#[test]
fn pid_without_derivative_matches_pi() {
    let mut pid = {
        let mut v = Pid::default();
        v.kd = [0.0; 2];
        v
    };
    let mut pi = gaden::control::Pi::default();
    let mut r = Rng(3);
    for _ in 0..1000 {
        let y = [30.0 * r.next_f64(), 12.0 * r.next_f64()];
        let a = pid.step(y, 0.1, UMAX);
        let b = pi.step(y, 0.1, UMAX);
        assert!((a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
    }
}

#[test]
fn pid_derivative_opposes_rising_agrp() {
    let mut a = {
        let mut v = Pid::default();
        v.ki = [0.0; 2];
        v.kp = [0.0; 2];
        v
    };
    a.step([5.0, 8.0], 1.0, UMAX);
    let u = a.step([10.0, 8.0], 1.0, UMAX);
    assert!(u[0] > 0.0); // rising AgRP -> more inhibition current
}

#[test]
fn pid_antiwindup_limits_integral() {
    // Saturate for a long time, then the error flips: the output must leave the limit fast.
    let mut c = Pid::default();
    for _ in 0..5000 {
        c.step([80.0, 8.0], 1.0, UMAX);
    }
    let mut steps = 0;
    while c.step([0.0, 8.0], 1.0, UMAX)[0] > 0.0 && steps < 1000 {
        steps += 1;
    }
    assert!(steps < 200, "took {steps} steps");
}

#[test]
fn mpc_solve_zero_when_at_target() {
    let m = Mpc::default();
    // x0 = target, d = target -> u = 0 is optimal (lambda > 0)
    assert!(m.solve(0, 1.0, 1.0, 1.0).abs() < 1e-9);
    assert!(m.solve(1, 8.0, 8.0, 1.0).abs() < 1e-9);
}

#[test]
fn mpc_solve_sign() {
    let m = Mpc::default();
    assert!(m.solve(0, 20.0, 20.0, 1.0) > 0.0); // AgRP too high: inhibit (g < 0, u > 0)
    assert!(m.solve(1, 3.0, 3.0, 1.0) > 0.0); // POMC too low: excite
}

#[test]
fn mpc_solve_steady_state_exact_for_long_horizon() {
    let m = {
        let mut v = Mpc::default();
        v.horizon = 2000;
        v.lambda = [0.0; 2];
        v
    };
    let u = m.solve(0, 21.0, 21.0, 1.0);
    // steady state d + g u = target -> u = (21 - 1) / 1
    assert!((u - 20.0).abs() < 0.05, "u {u}");
}

#[test]
fn mpc_lambda_shrinks_input() {
    let a = Mpc::default().solve(0, 20.0, 20.0, 1.0);
    let b = {
        let mut v = Mpc::default();
        v.lambda = [100.0, 100.0];
        v
    }
    .solve(0, 20.0, 20.0, 1.0);
    assert!(b < a);
}

#[test]
fn mpc_offset_free_under_gain_error() {
    // internal model gain is nominal; plant gain is 25% lower. Offset-free => still on target.
    let mut c = Mpc::default();
    let x = hold(&mut c, [15.0, 3.0], [0.75, 0.75], 1800);
    assert!(
        (x[0] - 1.0).abs() < 0.2 && (x[1] - 8.0).abs() < 0.2,
        "{x:?}"
    );
}

#[test]
fn bandit_tries_every_arm() {
    let mut b = Bandit::new(1);
    for _ in 0..(60 * 6) {
        b.step([10.0, 5.0], 1.0, UMAX);
    }
    assert!(b.counts().iter().all(|&n| n >= 1), "{:?}", b.counts());
}

#[test]
fn bandit_values_are_nonpositive() {
    let mut b = Bandit::new(2);
    for _ in 0..3600 {
        b.step([10.0, 5.0], 1.0, UMAX);
    }
    assert!(b.values().iter().all(|&q| q <= 0.0));
}

#[test]
fn bandit_is_seed_deterministic() {
    let run = |s| {
        let mut b = Bandit::new(s);
        let mut x = [20.0, 3.0];
        for _ in 0..3600 {
            let u = b.step(x, 1.0, UMAX);
            x = [
                x[0] + (20.0 - u[0] - x[0]) / 5.0,
                x[1] + (3.0 + 0.5 * u[1] - x[1]) / 5.0,
            ];
        }
        (b.counts().to_vec(), b.current_arm())
    };
    assert_eq!(run(11), run(11));
}

#[test]
fn bandit_window_counts_add_up() {
    let mut b = Bandit::new(4);
    for _ in 0..(60 * 30) {
        b.step([3.0, 7.0], 1.0, UMAX);
    }
    assert_eq!(b.counts().iter().sum::<u32>(), 30);
}

#[test]
fn bandit_prefers_better_arm_on_slow_plant() {
    // After 6 h on a plant with low gain, the greedy arm should not be the weakest (0.5x).
    let mut b = Bandit::new(9);
    hold(&mut b, [25.0, 3.0], [0.7, 0.7], 6 * 3600);
    assert_ne!(b.current_arm(), 0);
}
