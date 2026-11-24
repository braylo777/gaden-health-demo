//! C-ABI exports for the browser demo. No wasm-bindgen and no JavaScript glue crate:
//! JavaScript calls `run`, then reads `f64` arrays from linear memory at `out_ptr()`.
//! Simulation only. Not a medical device.

use gaden::controllers;
use gaden::hormones::{Coupling, Day, Dose, Ext, Meal, Params};
use gaden::sim::{self, Scenario};

const N: usize = 1440; // one sample per simulated minute
pub const CHANNELS: usize = 14;
const NAMES: [&str; 7] = ["pid", "mpc", "bandit", "rmpc", "safe_pid", "safe_rmpc", "open"];

static mut OUT: [f64; CHANNELS * N] = [0.0; CHANNELS * N];
static mut RES: [f64; 16] = [0.0; 16];
static MODEL: std::sync::OnceLock<(Params, Coupling)> = std::sync::OnceLock::new();

/// The default parameters (rounded fit values) and the default, already calibrated, coupling.
/// The metrics run refits and recalibrates; the difference is below 1% in w_ghr.
fn model() -> &'static (Params, Coupling) {
    MODEL.get_or_init(|| (Params::default(), Coupling::default()))
}

/// Number of samples per channel.
#[no_mangle]
pub extern "C" fn samples() -> u32 {
    N as u32
}

/// Number of channels. Order: t_h, agrp_open, agrp, pomc, u_agrp, u_pomc, drive_agrp, ghrelin,
/// leptin, insulin, glp1, pyy, cortisol, glucose.
#[no_mangle]
pub extern "C" fn channels() -> u32 {
    CHANNELS as u32
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const f64 {
    core::ptr::addr_of!(OUT) as *const f64
}

/// Result vector: [pass, frac_below_2hz, agrp_max, pomc_mean, u_mean_agrp, u_peak_agrp,
/// open_frac_below_2hz, open_agrp_max, open_pass, finite, bounded].
#[no_mangle]
pub extern "C" fn res_ptr() -> *const f64 {
    core::ptr::addr_of!(RES) as *const f64
}

/// Number of controllers and their index order: pid, mpc, bandit, rmpc, safe_pid, safe_rmpc, open.
#[no_mangle]
pub extern "C" fn n_controllers() -> u32 {
    NAMES.len() as u32
}

/// Run one day. Returns 1 on success, 0 on a bad controller index.
/// Inputs are clamped to the ranges in the demo.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn run(
    ctrl: u32,
    breakfast_g: f64,
    lunch_g: f64,
    dinner_g: f64,
    snack_g: f64,
    sleep_h: f64,
    stress: f64,
    circ_shift_h: f64,
    illness: f64,
    dose: f64,
    noise: f64,
    gain: f64,
    seed: u32,
) -> u32 {
    let Some(&name) = NAMES.get(ctrl as usize) else {
        return 0;
    };
    let (p, c) = model();
    let g = |v: f64, lo: f64, hi: f64| if v.is_finite() { v.clamp(lo, hi) } else { lo };
    let mut meals = vec![];
    for (at, grams) in [
        (8.0, g(breakfast_g, 0.0, 300.0)),
        (12.5, g(lunch_g, 0.0, 300.0)),
        (18.5, g(dinner_g, 0.0, 300.0)),
        (22.5, g(snack_g, 0.0, 300.0)),
    ] {
        if grams > 0.0 {
            meals.push(Meal {
                at_h: at,
                carbs_g: grams,
            });
        }
    }
    let doses = if dose > 0.0 {
        vec![Dose {
            at_h: 8.0,
            units: g(dose, 0.0, 3.0),
        }]
    } else {
        vec![]
    };
    let gain = g(gain, 0.5, 1.5);
    let sc = Scenario {
        name: "demo".into(),
        category: "demo",
        warm: Day {
            meals: meals.clone(),
            doses: doses.clone(),
            sleep_h: 8.0,
        },
        day: Day {
            meals,
            doses,
            sleep_h: g(sleep_h, 0.0, 12.0),
        },
        gain_err: [gain, gain],
        noise: g(noise, 0.0, 3.0),
        seed: seed as u64 + 1,
        ext: Ext {
            circ_shift_h: g(circ_shift_h, -12.0, 12.0),
            stress: g(stress, 0.0, 1.0),
            illness: g(illness, 0.0, 1.0),
        },
    };
    // open loop for reference, then the chosen controller (open = same curve twice)
    let (ro, so) = sim::run_day_series(&sc, None, p, c, 60);
    let (rc, sc_) = if name == "open" {
        (ro.clone(), so.clone())
    } else {
        let mut k = controllers::by_name(name, sc.seed).unwrap();
        sim::run_day_series(&sc, Some(k.as_mut()), p, c, 60)
    };
    unsafe {
        let chans: [&Vec<f64>; CHANNELS] = [
            &sc_.t_h,
            &so.agrp,
            &sc_.agrp,
            &sc_.pomc,
            &sc_.u_agrp,
            &sc_.u_pomc,
            &sc_.drive_agrp,
            &sc_.ghrelin,
            &sc_.leptin,
            &sc_.insulin,
            &sc_.glp1,
            &sc_.pyy,
            &sc_.cortisol,
            &sc_.glucose,
        ];
        for (i, ch) in chans.iter().enumerate() {
            for k in 0..N {
                OUT[i * N + k] = ch.get(k).copied().unwrap_or(0.0);
            }
        }
        RES = [
            rc.pass() as u8 as f64,
            rc.frac_agrp_below_2hz,
            rc.agrp_max_hz,
            rc.pomc_mean_hz,
            rc.u_mean[0],
            rc.u_peak[0],
            ro.frac_agrp_below_2hz,
            ro.agrp_max_hz,
            ro.pass() as u8 as f64,
            rc.finite as u8 as f64,
            rc.bounded as u8 as f64,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
    }
    1
}
