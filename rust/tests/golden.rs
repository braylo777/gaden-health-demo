//! Golden curves: the model output on the standard day (day 2, every 2 h) must match the stored
//! snapshot `fixtures/golden_curves.json`. The snapshot is written by `examples/golden.rs`.
//! It guards against silent model drift. It is a regression guard, not a data fit.

use gaden::hormones::{run, Day, Params, HOUR};
use serde_json::Value;

fn fx() -> Value {
    serde_json::from_str(include_str!("fixtures/golden_curves.json")).unwrap()
}

fn check(var: &str, k: usize) {
    let f = fx();
    let h = f["hours"][k].as_f64().unwrap();
    let want = f[var][k].as_f64().unwrap();
    let tr = run(&Params::default(), &Day::standard(), 48.0, 60.0);
    let i = ((h * HOUR / 60.0).round() as usize).min(tr.s.len() - 1);
    let s = &tr.s[i];
    let got = match var {
        "ghrelin" => s.ghrelin,
        "leptin" => s.leptin,
        "insulin" => s.insulin,
        "glp1" => s.glp1,
        "glucose" => s.glucose,
        "pyy" => s.pyy,
        "cortisol" => s.cortisol,
        _ => unreachable!(),
    };
    assert!(
        (got - want).abs() <= 1e-6 * (1.0 + want.abs()),
        "{var} at hour {h}: got {got}, golden {want}"
    );
}

macro_rules! golden {
    ($($name:ident: ($var:expr, $k:expr),)*) => {
        $( #[test] fn $name() { check($var, $k); } )*
    };
}

golden! {
    golden_ghrelin_h24: ("ghrelin", 0),
    golden_ghrelin_h26: ("ghrelin", 1),
    golden_ghrelin_h28: ("ghrelin", 2),
    golden_ghrelin_h30: ("ghrelin", 3),
    golden_ghrelin_h32: ("ghrelin", 4),
    golden_ghrelin_h34: ("ghrelin", 5),
    golden_ghrelin_h36: ("ghrelin", 6),
    golden_ghrelin_h38: ("ghrelin", 7),
    golden_ghrelin_h40: ("ghrelin", 8),
    golden_ghrelin_h42: ("ghrelin", 9),
    golden_ghrelin_h44: ("ghrelin", 10),
    golden_ghrelin_h46: ("ghrelin", 11),
    golden_leptin_h24: ("leptin", 0),
    golden_leptin_h26: ("leptin", 1),
    golden_leptin_h28: ("leptin", 2),
    golden_leptin_h30: ("leptin", 3),
    golden_leptin_h32: ("leptin", 4),
    golden_leptin_h34: ("leptin", 5),
    golden_leptin_h36: ("leptin", 6),
    golden_leptin_h38: ("leptin", 7),
    golden_leptin_h40: ("leptin", 8),
    golden_leptin_h42: ("leptin", 9),
    golden_leptin_h44: ("leptin", 10),
    golden_leptin_h46: ("leptin", 11),
    golden_insulin_h24: ("insulin", 0),
    golden_insulin_h26: ("insulin", 1),
    golden_insulin_h28: ("insulin", 2),
    golden_insulin_h30: ("insulin", 3),
    golden_insulin_h32: ("insulin", 4),
    golden_insulin_h34: ("insulin", 5),
    golden_insulin_h36: ("insulin", 6),
    golden_insulin_h38: ("insulin", 7),
    golden_insulin_h40: ("insulin", 8),
    golden_insulin_h42: ("insulin", 9),
    golden_insulin_h44: ("insulin", 10),
    golden_insulin_h46: ("insulin", 11),
    golden_glp1_h24: ("glp1", 0),
    golden_glp1_h26: ("glp1", 1),
    golden_glp1_h28: ("glp1", 2),
    golden_glp1_h30: ("glp1", 3),
    golden_glp1_h32: ("glp1", 4),
    golden_glp1_h34: ("glp1", 5),
    golden_glp1_h36: ("glp1", 6),
    golden_glp1_h38: ("glp1", 7),
    golden_glp1_h40: ("glp1", 8),
    golden_glp1_h42: ("glp1", 9),
    golden_glp1_h44: ("glp1", 10),
    golden_glp1_h46: ("glp1", 11),
    golden_glucose_h24: ("glucose", 0),
    golden_glucose_h26: ("glucose", 1),
    golden_glucose_h28: ("glucose", 2),
    golden_glucose_h30: ("glucose", 3),
    golden_glucose_h32: ("glucose", 4),
    golden_glucose_h34: ("glucose", 5),
    golden_glucose_h36: ("glucose", 6),
    golden_glucose_h38: ("glucose", 7),
    golden_glucose_h40: ("glucose", 8),
    golden_glucose_h42: ("glucose", 9),
    golden_glucose_h44: ("glucose", 10),
    golden_glucose_h46: ("glucose", 11),
    golden_pyy_h24: ("pyy", 0),
    golden_pyy_h26: ("pyy", 1),
    golden_pyy_h28: ("pyy", 2),
    golden_pyy_h30: ("pyy", 3),
    golden_pyy_h32: ("pyy", 4),
    golden_pyy_h34: ("pyy", 5),
    golden_pyy_h36: ("pyy", 6),
    golden_pyy_h38: ("pyy", 7),
    golden_pyy_h40: ("pyy", 8),
    golden_pyy_h42: ("pyy", 9),
    golden_pyy_h44: ("pyy", 10),
    golden_pyy_h46: ("pyy", 11),
    golden_cortisol_h24: ("cortisol", 0),
    golden_cortisol_h26: ("cortisol", 1),
    golden_cortisol_h28: ("cortisol", 2),
    golden_cortisol_h30: ("cortisol", 3),
    golden_cortisol_h32: ("cortisol", 4),
    golden_cortisol_h34: ("cortisol", 5),
    golden_cortisol_h36: ("cortisol", 6),
    golden_cortisol_h38: ("cortisol", 7),
    golden_cortisol_h40: ("cortisol", 8),
    golden_cortisol_h42: ("cortisol", 9),
    golden_cortisol_h44: ("cortisol", 10),
    golden_cortisol_h46: ("cortisol", 11),
}
