//! Print the stress-suite failures per controller: `cargo run --release --example failures`.
use gaden::{controllers, hormones::*, sim};

fn main() {
    let (p, c) = (Params::default(), Coupling::default());
    let s = sim::suite();
    for n in controllers::NAMES {
        let r = sim::run_suite(n, &s, &p, &c);
        println!(
            "{n}: {}/{} pass; failures: {}",
            r.passed,
            r.n,
            r.failures.join(" ")
        );
    }
}
