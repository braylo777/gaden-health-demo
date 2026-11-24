//! Writes `tests/fixtures/golden_curves.json`: the hormone curves of the standard day, day 2,
//! every 2 h, for the default parameters. Regenerate only when the model changes on purpose:
//! `cargo run --release --example golden`.

use gaden::hormones::{run, Day, Params, State, HOUR};

type Getter = fn(&State) -> f64;
use std::fmt::Write as _;

fn main() {
    let p = Params::default();
    let tr = run(&p, &Day::standard(), 48.0, 60.0);
    let mut j = String::from("{\n  \"hours\": [");
    let hours: Vec<f64> = (0..12).map(|k| 24.0 + 2.0 * k as f64).collect();
    j.push_str(
        &hours
            .iter()
            .map(|h| format!("{h}"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    j.push_str("],\n");
    let vars: [(&str, Getter); 7] = [
        ("ghrelin", |s| s.ghrelin),
        ("leptin", |s| s.leptin),
        ("insulin", |s| s.insulin),
        ("glp1", |s| s.glp1),
        ("glucose", |s| s.glucose),
        ("pyy", |s| s.pyy),
        ("cortisol", |s| s.cortisol),
    ];
    for (vi, (name, f)) in vars.iter().enumerate() {
        let vals: Vec<String> = hours
            .iter()
            .map(|h| {
                let i = ((h * HOUR / 60.0).round() as usize).min(tr.s.len() - 1);
                format!("{:.9}", f(&tr.s[i]))
            })
            .collect();
        let _ = writeln!(
            j,
            "  \"{name}\": [{}]{}",
            vals.join(", "),
            if vi + 1 < vars.len() { "," } else { "" }
        );
    }
    j.push_str("}\n");
    std::fs::write("tests/fixtures/golden_curves.json", j).expect("write");
}
