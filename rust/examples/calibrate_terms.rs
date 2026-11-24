//! Prints the three literature anchors that set `drug_empt_slow`, `misalign_isens` and the
//! cortisol awakening timing. `cargo run --release --example calibrate_terms`

use gaden::{anchors, hormones::Params};

fn main() {
    let p = Params::default();
    println!(
        "first-hour gastric emptying, agonist / none: {:.3}  (Hjerpsted 2018: 0.73, CI 0.61-0.87)",
        anchors::first_hour_emptying_ratio(&p)
    );
    println!(
        "postprandial glucose, 12 h misaligned / aligned: {:.3}  (Morris 2015: 1.06)",
        anchors::misalign_glucose_ratio(&p)
    );
    println!(
        "cortisol peak after waking: {:.0} min  (Stalder 2016: 30-45 min)",
        anchors::car_peak_minutes(&p)
    );
}
