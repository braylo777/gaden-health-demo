//! Property tests: random inputs from a seeded generator, many cases per property.
//! No external crate; failures print the case so it can be replayed.

use gaden::control::{eigen_real_parts, Pi, Plant, Rng};
use gaden::controllers::{self, Controller, Mpc, Pid};
use gaden::hormones::*;
use gaden::sim::{self, Scenario};

fn uni(r: &mut Rng, a: f64, b: f64) -> f64 {
    a + (b - a) * r.next_f64()
}

fn random_day(r: &mut Rng) -> Day {
    let n = (r.next_f64() * 6.0) as usize;
    Day {
        meals: (0..n)
            .map(|_| Meal {
                at_h: uni(r, 0.0, 24.0),
                carbs_g: uni(r, 0.0, 250.0),
            })
            .collect(),
        doses: if r.next_f64() < 0.5 {
            vec![Dose {
                at_h: uni(r, 0.0, 24.0),
                units: uni(r, 0.0, 3.0),
            }]
        } else {
            vec![]
        },
        sleep_h: uni(r, 0.0, 12.0),
    }
}

#[test]
fn prop_hormones_bounded_and_positive() {
    let mut r = Rng(101);
    let p = Params::default();
    for case in 0..60 {
        let d = random_day(&mut r);
        let tr = run(&p, &d, 48.0, 60.0);
        for s in &tr.s {
            assert!(s.is_finite(), "case {case}: {d:?}");
            assert!(
                s.ghrelin > 0.0 && s.ghrelin < 10.0 * p.ghr_base,
                "case {case}"
            );
            assert!(s.leptin > 0.0 && s.leptin < 3.0 * p.lep_mean, "case {case}");
            assert!(s.glucose >= 20.0 && s.glucose < 600.0, "case {case}");
            assert!(s.insulin > 0.0 && s.insulin < 1000.0, "case {case}");
        }
    }
}

#[test]
fn prop_neural_drive_in_range() {
    let mut r = Rng(102);
    let (p, c) = (Params::default(), Coupling::default());
    for _ in 0..5000 {
        let mut s = State::basal(&p);
        s.ghrelin = uni(&mut r, 0.0, 5000.0);
        s.leptin = uni(&mut r, 0.0, 60.0);
        s.insulin = uni(&mut r, 0.0, 300.0);
        s.glp1 = uni(&mut r, 0.0, 200.0);
        s.drug = uni(&mut r, 0.0, 5.0);
        let d = neural_drive(&s, &p, &c);
        assert!(d.iter().all(|v| (0.0..=120.0).contains(v)));
    }
}

#[test]
fn prop_linear_loop_stable_over_gain_range() {
    // Stability of the linearised PI loop for every plant gain error in [0.5, 2.0].
    let mut r = Rng(103);
    for _ in 0..2000 {
        let p = Plant {
            gain: [-uni(&mut r, 0.5, 2.0), uni(&mut r, 0.25, 1.0)],
            tau: uni(&mut r, 1.0, 20.0),
            ..Plant::default()
        };
        for ch in eigen_real_parts(&p, &Pi::default()) {
            assert!(ch[0] < 0.0 && ch[1] < 0.0, "{p:?}");
        }
    }
}

#[test]
fn prop_closed_loop_bounded_random_scenarios() {
    let mut r = Rng(104);
    let (p, c) = (Params::default(), Coupling::default());
    for case in 0..24 {
        let sc = Scenario {
            name: format!("rand{case}"),
            category: "random",
            warm: random_day(&mut r),
            day: random_day(&mut r),
            gain_err: [uni(&mut r, 0.5, 1.5), uni(&mut r, 0.5, 1.5)],
            noise: uni(&mut r, 0.0, 1.0),
            seed: case,
        };
        let name = controllers::NAMES[case as usize % 3];
        let mut k = controllers::by_name(name, case).unwrap();
        let out = sim::run_day(&sc, Some(k.as_mut()), &p, &c);
        assert!(out.finite && out.bounded, "{name} {sc:?}");
        assert!(out.agrp_max_hz <= sim::X_BOUND && out.pomc_mean_hz >= 0.0);
    }
}

#[test]
fn prop_controller_bounded_input_bounded_output() {
    // BIBO on the plant: any bounded drive gives bounded rates for every controller.
    let mut r = Rng(105);
    for case in 0..30 {
        let mut k = controllers::by_name(controllers::NAMES[case % 3], case as u64).unwrap();
        let p = Plant::default();
        let mut x = [0.0f64; 2];
        let mut xmax = 0.0f64;
        for _ in 0..3000 {
            let d = [uni(&mut r, 0.0, 120.0), uni(&mut r, 0.0, 120.0)];
            let u = k.step(x, 1.0, p.u_max);
            for i in 0..2 {
                x[i] = (x[i] + (d[i] + p.gain[i] * u[i] - x[i]) / p.tau).max(0.0);
            }
            xmax = xmax.max(x[0]).max(x[1]);
        }
        assert!(xmax <= 120.0 + 1e-9, "case {case}: {xmax}");
    }
}

#[test]
fn prop_more_current_never_raises_agrp() {
    // Monotone actuator: for the AgRP channel, more inhibition current gives lower next state.
    let mut r = Rng(106);
    let p = Plant::default();
    for _ in 0..5000 {
        let (x, d) = (uni(&mut r, 0.0, 60.0), uni(&mut r, 0.0, 60.0));
        let (u1, u2) = (uni(&mut r, 0.0, 40.0), uni(&mut r, 0.0, 40.0));
        let nx = |u: f64| x + (d + p.gain[0] * u - x) / p.tau;
        if u1 <= u2 {
            assert!(nx(u2) <= nx(u1) + 1e-12);
        }
    }
}

#[test]
fn prop_mpc_solution_finite() {
    let mut r = Rng(107);
    let m = Mpc::default();
    for _ in 0..5000 {
        let u = m.solve(
            (r.next_f64() * 2.0) as usize % 2,
            uni(&mut r, 0.0, 150.0),
            uni(&mut r, -50.0, 150.0),
            uni(&mut r, 0.01, 4.0),
        );
        assert!(u.is_finite());
    }
}

#[test]
fn prop_pid_converges_from_random_start() {
    let mut r = Rng(108);
    for case in 0..30 {
        let d = [uni(&mut r, 5.0, 25.0), uni(&mut r, 0.0, 6.0)];
        let mut x = [uni(&mut r, 0.0, 50.0), uni(&mut r, 0.0, 20.0)];
        let p = Plant::default();
        let mut k = Pid::default();
        for _ in 0..1800 {
            let u = k.step(x, 1.0, p.u_max);
            for i in 0..2 {
                x[i] = (x[i] + (d[i] + p.gain[i] * u[i] - x[i]) / p.tau).max(0.0);
            }
        }
        assert!(
            (x[0] - 1.0).abs() < 0.1 && (x[1] - 8.0).abs() < 0.1,
            "case {case}: {d:?} -> {x:?}"
        );
    }
}

#[test]
fn prop_mpc_converges_from_random_start() {
    let mut r = Rng(109);
    for case in 0..30 {
        let d = [uni(&mut r, 5.0, 25.0), uni(&mut r, 0.0, 6.0)];
        let ge = [uni(&mut r, 0.7, 1.3), uni(&mut r, 0.7, 1.3)];
        let mut x = [uni(&mut r, 0.0, 50.0), uni(&mut r, 0.0, 20.0)];
        let p = Plant::default();
        let mut k = Mpc::default();
        for _ in 0..1800 {
            let u = k.step(x, 1.0, p.u_max);
            for i in 0..2 {
                x[i] = (x[i] + (d[i] + p.gain[i] * ge[i] * u[i] - x[i]) / p.tau).max(0.0);
            }
        }
        assert!(
            (x[0] - 1.0).abs() < 0.2 && (x[1] - 8.0).abs() < 0.2,
            "case {case}: {d:?} {ge:?} -> {x:?}"
        );
    }
}

#[test]
fn prop_leptin_shape_bounded() {
    let mut r = Rng(110);
    let p = Params::default();
    for _ in 0..5000 {
        let v = leptin_shape(&p, uni(&mut r, -100.0, 100.0));
        assert!(v >= 1.0 - p.lep_a1 - p.lep_a2 - 1e-12 && v <= 1.0 + p.lep_a1 + p.lep_a2 + 1e-12);
    }
}

#[test]
fn prop_meal_mass_conserved() {
    // Without absorption, stomach + gut carbs are conserved: emptying only moves mass.
    let mut r = Rng(111);
    let p = Params {
        gut_k: 0.0,
        ..Params::default()
    };
    for _ in 0..50 {
        let g = uni(&mut r, 1.0, 200.0);
        let mut s = State::basal(&p);
        eat(
            &mut s,
            &Meal {
                at_h: 0.0,
                carbs_g: g,
            },
        );
        for k in 0..600 {
            step(&mut s, &p, k as f64 / 60.0, 60.0, 8.0);
        }
        assert!((s.stomach + s.gut - g).abs() < 1e-6 * g);
    }
}

#[test]
fn prop_bandit_arm_index_valid() {
    let mut r = Rng(112);
    for seed in 0..20 {
        let mut b = controllers::Bandit::new(seed);
        for _ in 0..3600 {
            b.step(
                [uni(&mut r, 0.0, 30.0), uni(&mut r, 0.0, 15.0)],
                1.0,
                [40.0, 25.0],
            );
            assert!(b.current_arm() < b.arms.len());
        }
    }
}
