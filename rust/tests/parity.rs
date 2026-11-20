//! Parity with the Python package. Fixture written by Python (`rust/tests/fixtures/parity.json`).

use gaden::{control, neuron, pomc};
use serde_json::Value;

fn fx() -> Value {
    serde_json::from_str(include_str!("fixtures/parity.json")).unwrap()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + b.abs())
}

#[test]
fn neuron_matches_python() {
    for c in fx()["neuron"].as_array().unwrap() {
        let i = c["i"].as_f64().unwrap();
        let r = neuron::simulate_dc(i, 100.0);
        assert_eq!(
            r.spikes as i64,
            c["spikes"].as_i64().unwrap(),
            "spikes at I={i}"
        );
        assert_eq!(
            neuron::is_blocked(&r, 0.3),
            c["blocked"].as_bool().unwrap(),
            "blocked at I={i}"
        );
        for (k, want) in c["v"].as_array().unwrap().iter().enumerate() {
            let got = r.v[k * 500];
            assert!(
                close(got, want.as_f64().unwrap(), 1e-9),
                "V at I={i}, sample {k}: {got} vs {want}"
            );
        }
        assert!(close(
            *r.h.last().unwrap(),
            c["h_end"].as_f64().unwrap(),
            1e-9
        ));
    }
}

#[test]
fn pomc_matches_python() {
    let f = fx();
    let p = &f["pomc"];
    let got = pomc::pulse_times_ms(3000.0);
    let want: Vec<f64> = p["pulses_3000"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    assert_eq!(got.len(), want.len());
    assert!(got.iter().zip(&want).all(|(a, b)| close(*a, *b, 1e-12)));
    let env: Vec<bool> = [0.0, 250.0, 499.9, 500.0, 1499.0, 1500.0, 2100.0]
        .iter()
        .map(|t| pomc::burst_envelope(*t))
        .collect();
    let want_env: Vec<bool> = p["env"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_bool().unwrap())
        .collect();
    assert_eq!(env, want_env);
    assert!(close(
        pomc::charge_per_phase_nc(),
        p["charge"].as_f64().unwrap(),
        1e-12
    ));
    assert!(close(
        pomc::duty_cycle(),
        p["duty"].as_f64().unwrap(),
        1e-12
    ));
}

#[test]
fn controller_matches_python_noise_free() {
    for c in fx()["control"].as_array().unwrap() {
        let ge = [
            c["gain_err"][0].as_f64().unwrap(),
            c["gain_err"][1].as_f64().unwrap(),
        ];
        let s = control::simulate(
            c["closed"].as_bool().unwrap(),
            c["peak"].as_f64().unwrap(),
            ge,
            0.0,
            0,
        );
        for (k, row) in c["x"].as_array().unwrap().iter().enumerate() {
            for j in 0..2 {
                assert!(
                    close(s.x[k * 100][j], row[j].as_f64().unwrap(), 1e-9),
                    "x[{k}][{j}]"
                );
                assert!(
                    close(s.u[k * 100][j], c["u"][k][j].as_f64().unwrap(), 1e-9),
                    "u[{k}][{j}]"
                );
            }
        }
        let m = control::metrics(&s, 120.0);
        let w = &c["metrics"];
        assert!(close(
            m.agrp_max_hz,
            w["agrp_max_hz"].as_f64().unwrap(),
            1e-9
        ));
        assert!(close(
            m.pomc_mean_hz,
            w["pomc_mean_hz"].as_f64().unwrap(),
            1e-9
        ));
        assert!(close(
            m.frac_agrp_below_2hz,
            w["frac_agrp_below_2hz"].as_f64().unwrap(),
            1e-12
        ));
    }
}

#[test]
fn eigenvalues_match_python() {
    let f = fx();
    let got = control::eigen_real_parts(&control::Plant::default(), &control::Pi::default());
    for (ch, want) in f["eig"].as_array().unwrap().iter().enumerate() {
        for j in 0..2 {
            assert!(close(got[ch][j], want[j].as_f64().unwrap(), 1e-9));
            assert!(got[ch][j] < 0.0, "unstable");
        }
    }
}

#[test]
fn behaviour_matches_python_tests() {
    assert_eq!(neuron::simulate_dc(0.0, 100.0).spikes, 0);
    assert!(neuron::simulate_dc(20.0, 300.0).spikes > 10);
    let b = neuron::simulate_dc(300.0, 300.0);
    assert!(neuron::is_blocked(&b, 0.3) && *b.h.last().unwrap() < 0.1);
    let r = control::simulate(true, 60.0, [1.0, 1.0], 0.3, 7);
    assert!(r.u.iter().all(|u| u[0] <= 40.0 && u[1] <= 25.0));
    let (pass, _) = control::monte_carlo(200, 1);
    assert!(pass > 0.85, "stress pass rate {pass}");
}
