//! Which safety limit costs the most pass rate? Runs PID behind the filter with one limit relaxed
//! at a time, on the 1,000-scenario suite. `cargo run --release --example ablate_safety`

use gaden::controllers::*;
use gaden::hormones::{Coupling, Params};
use gaden::sim;

fn pass(lim: SafetyLimits, suite: &[sim::Scenario], p: &Params, c: &Coupling) -> f64 {
    let mut ok = 0;
    for sc in suite {
        let mut k = Safe::new(Box::new(Pid::default()), lim.clone(), "safe_pid");
        ok += sim::run_day(sc, Some(&mut k), p, c).pass() as usize;
    }
    ok as f64 / suite.len() as f64
}

fn main() {
    let (p, c) = (Params::default(), Coupling::default());
    let suite = sim::suite_1000();
    let d = SafetyLimits::default();
    let big = 1e9;
    let cases = [
        ("all limits", d.clone()),
        (
            "no running-mean limit",
            SafetyLimits {
                mean_limit: [big; 2],
                ..d.clone()
            },
        ),
        (
            "no rise-rate limit",
            SafetyLimits {
                slew: [big; 2],
                ..d.clone()
            },
        ),
        (
            "peak = device limit",
            SafetyLimits {
                peak: [40.0, 25.0],
                ..d.clone()
            },
        ),
        (
            "only peak",
            SafetyLimits {
                mean_limit: [big; 2],
                slew: [big; 2],
                ..d.clone()
            },
        ),
    ];
    for (name, lim) in cases {
        println!("{name:24} pass {:.1}%", 100.0 * pass(lim, &suite, &p, &c));
    }
}
