//! Writes `../results/figs.json`: data for the figures in `docs/img/` (nominal day curves and
//! four failure traces of the PID controller). `cargo run --release --example export_figs`

use gaden::controllers;
use gaden::hormones::{Coupling, Params};
use gaden::sim::{self, Series};
use std::fmt::Write as _;

fn arr(v: &[f64]) -> String {
    let s: Vec<String> = v.iter().map(|x| format!("{:.3}", x)).collect();
    format!("[{}]", s.join(","))
}

fn series_json(se: &Series) -> String {
    format!(
        "{{\"t_h\":{},\"agrp\":{},\"pomc\":{},\"drive_agrp\":{},\"u_agrp\":{},\"u_pomc\":{},\"ghrelin\":{},\"leptin\":{},\"insulin\":{},\"glp1\":{},\"pyy\":{},\"cortisol\":{},\"glucose\":{}}}",
        arr(&se.t_h), arr(&se.agrp), arr(&se.pomc), arr(&se.drive_agrp), arr(&se.u_agrp),
        arr(&se.u_pomc), arr(&se.ghrelin), arr(&se.leptin), arr(&se.insulin), arr(&se.glp1),
        arr(&se.pyy), arr(&se.cortisol), arr(&se.glucose)
    )
}

fn main() {
    // use the same fitted parameters as the metrics run
    let ts = gaden::fit::targets();
    let p = gaden::fit::fit(&Params::prior(), &ts, 600).params;
    let c = sim::calibrate_coupling(&p, &Coupling::default(), 30.0);
    let mut j = String::from("{\n");
    // nominal day: open loop and PID
    let nom = sim::nominal();
    let (_, open) = sim::run_day_series(&nom, None, &p, &c, 300);
    let mut pid = controllers::by_name("pid", 1).unwrap();
    let (_, closed) = sim::run_day_series(&nom, Some(pid.as_mut()), &p, &c, 300);
    let _ = writeln!(j, "\"nominal_open\": {},", series_json(&open));
    let _ = writeln!(j, "\"nominal_pid\": {},", series_json(&closed));
    // failures: the worst failed PID scenario (lowest share of the day below 2 Hz) of four kinds
    let suite = sim::suite_1000();
    let mut fails = vec![];
    for kind in ["shift_work", "travel", "missed_meals", "sensor_noise"] {
        let mut worst: Option<(f64, &sim::Scenario)> = None;
        for sc in suite.iter().filter(|s| s.category == kind) {
            let mut k = controllers::by_name("pid", sc.seed).unwrap();
            let r = sim::run_day(sc, Some(k.as_mut()), &p, &c);
            if !r.pass() && worst.is_none_or(|w| r.frac_agrp_below_2hz < w.0) {
                worst = Some((r.frac_agrp_below_2hz, sc));
            }
        }
        if let Some((_, sc)) = worst {
            let mut k = controllers::by_name("pid", sc.seed).unwrap();
            let (r, se) = sim::run_day_series(sc, Some(k.as_mut()), &p, &c, 300);
            fails.push(format!(
                "{{\"name\":\"{}\",\"kind\":\"{}\",\"frac_below_2hz\":{:.4},\"pomc_mean\":{:.3},\"agrp_max\":{:.2},\"series\":{}}}",
                sc.name, kind, r.frac_agrp_below_2hz, r.pomc_mean_hz, r.agrp_max_hz, series_json(&se)
            ));
        }
    }
    let _ = writeln!(j, "\"failures\": [{}]", fails.join(",\n"));
    j.push_str("}\n");
    std::fs::create_dir_all("../results").ok();
    std::fs::write("../results/figs.json", j).expect("write figs.json");
}
