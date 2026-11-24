//! `gaden` command line: run a simulated day or the first N scenarios of the stress suite.
//!   gaden day [controller]        one nominal day (controller: pid, mpc, bandit, rmpc, safe_pid, safe_rmpc, open)
//!   gaden suite [N] [controller]  the first N scenarios (default 20) of the 1,000-scenario suite
//!   gaden --help
//! Simulation only. Not a medical device.

use gaden::controllers;
use gaden::hormones::{Coupling, Params};
use gaden::sim;

const HELP: &str = "gaden: closed-loop hunger-circuit simulation. Simulation only. Not a medical device.\n\
usage:\n  gaden day [controller]\n  gaden suite [N] [controller]\nControllers: pid mpc bandit rmpc safe_pid safe_rmpc open";

fn run_one(name: &str, sc: &sim::Scenario, p: &Params, c: &Coupling) -> Option<sim::DayResult> {
    if name == "open" {
        return Some(sim::run_day(sc, None, p, c));
    }
    let mut k = controllers::by_name(name, sc.seed)?;
    Some(sim::run_day(sc, Some(k.as_mut()), p, c))
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (p, c) = (Params::default(), Coupling::default());
    match a.first().map(String::as_str) {
        Some("day") => {
            let name = a.get(1).map(String::as_str).unwrap_or("pid");
            match run_one(name, &sim::nominal(), &p, &c) {
                Some(r) => {
                    println!("controller {name}, nominal day");
                    println!(
                        "AgRP peak {:.2} Hz, below 2 Hz for {:.1}% of the day",
                        r.agrp_max_hz,
                        100.0 * r.frac_agrp_below_2hz
                    );
                    println!(
                        "POMC mean {:.2} Hz (target 8), mean AgRP current {:.1} uA",
                        r.pomc_mean_hz, r.u_mean[0]
                    );
                    println!("day result: {}", if r.pass() { "pass" } else { "not pass" });
                }
                None => {
                    eprintln!("unknown controller '{name}'\n{HELP}");
                    std::process::exit(2);
                }
            }
        }
        Some("suite") => {
            let n: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
            let name = a.get(2).map(String::as_str).unwrap_or("pid");
            let suite = sim::suite_1000();
            let n = n.min(suite.len());
            let mut ok = 0;
            for sc in &suite[..n] {
                match run_one(name, sc, &p, &c) {
                    Some(r) => ok += r.pass() as usize,
                    None => {
                        eprintln!("unknown controller '{name}'\n{HELP}");
                        std::process::exit(2);
                    }
                }
            }
            let (lo, hi) = sim::wilson_ci(ok, n);
            println!("controller {name}: {ok} of {n} scenarios pass ({:.1}%, 95% interval {:.1} to {:.1}%)", 100.0 * ok as f64 / n as f64, 100.0 * lo, 100.0 * hi);
        }
        _ => println!("{HELP}"),
    }
}
