//! More parity with the Python reference. Fixture `fixtures/parity2.json` is written by
//! `scripts/gen_parity2.py` (noise-free runs only). Each case is its own test.

use gaden::{control, neuron};
use serde_json::Value;

fn fx() -> Value {
    serde_json::from_str(include_str!("fixtures/parity2.json")).unwrap()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + b.abs())
}

fn neuron_case(idx: usize) {
    let f = fx();
    let c = &f["neuron"][idx];
    let i = c["i"].as_f64().unwrap();
    let r = neuron::simulate_dc(i, 60.0);
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
        assert!(
            close(r.v[k * 500], want.as_f64().unwrap(), 1e-9),
            "V at I={i}, sample {k}"
        );
    }
    assert!(close(
        *r.h.last().unwrap(),
        c["h_end"].as_f64().unwrap(),
        1e-9
    ));
}

fn control_case(idx: usize) {
    let f = fx();
    let c = &f["control"][idx];
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
                "x[{k}][{j}] case {idx}"
            );
            assert!(
                close(s.u[k * 100][j], c["u"][k][j].as_f64().unwrap(), 1e-9),
                "u[{k}][{j}] case {idx}"
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

macro_rules! neuron_cases {
    ($($name:ident: $i:expr,)*) => { $( #[test] fn $name() { neuron_case($i); } )* };
}
macro_rules! control_cases {
    ($($name:ident: $i:expr,)*) => { $( #[test] fn $name() { control_case($i); } )* };
}

neuron_cases! {
    parity2_neuron_00: 0,
    parity2_neuron_01: 1,
    parity2_neuron_02: 2,
    parity2_neuron_03: 3,
    parity2_neuron_04: 4,
    parity2_neuron_05: 5,
    parity2_neuron_06: 6,
    parity2_neuron_07: 7,
    parity2_neuron_08: 8,
    parity2_neuron_09: 9,
    parity2_neuron_10: 10,
    parity2_neuron_11: 11,
    parity2_neuron_12: 12,
    parity2_neuron_13: 13,
    parity2_neuron_14: 14,
    parity2_neuron_15: 15,
}

control_cases! {
    parity2_control_00: 0,
    parity2_control_01: 1,
    parity2_control_02: 2,
    parity2_control_03: 3,
    parity2_control_04: 4,
    parity2_control_05: 5,
    parity2_control_06: 6,
    parity2_control_07: 7,
    parity2_control_08: 8,
    parity2_control_09: 9,
    parity2_control_10: 10,
    parity2_control_11: 11,
    parity2_control_12: 12,
    parity2_control_13: 13,
    parity2_control_14: 14,
    parity2_control_15: 15,
    parity2_control_16: 16,
    parity2_control_17: 17,
    parity2_control_18: 18,
    parity2_control_19: 19,
    parity2_control_20: 20,
    parity2_control_21: 21,
    parity2_control_22: 22,
    parity2_control_23: 23,
    parity2_control_24: 24,
    parity2_control_25: 25,
    parity2_control_26: 26,
    parity2_control_27: 27,
    parity2_control_28: 28,
    parity2_control_29: 29,
    parity2_control_30: 30,
    parity2_control_31: 31,
    parity2_control_32: 32,
    parity2_control_33: 33,
    parity2_control_34: 34,
    parity2_control_35: 35,
    parity2_control_36: 36,
    parity2_control_37: 37,
    parity2_control_38: 38,
    parity2_control_39: 39,
    parity2_control_40: 40,
    parity2_control_41: 41,
    parity2_control_42: 42,
    parity2_control_43: 43,
    parity2_control_44: 44,
    parity2_control_45: 45,
    parity2_control_46: 46,
    parity2_control_47: 47,
}
