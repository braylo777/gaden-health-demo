//! Multi-hormone appetite model: ghrelin, leptin, insulin, GLP-1, PYY, cortisol, glucose,
//! gastric emptying and cumulative sleep debt, with circadian timing.
//!
//! Time unit: seconds. Clock hour = (t / 3600 + start_hour) mod 24.
//! Every rate below is a model assumption unless `fit.rs` fits it to `data/targets.csv`.
//! This is a toy for control research. It is not a physiological model you can use for care.

use std::f64::consts::PI;

pub const HOUR: f64 = 3600.0;
pub const DAY: f64 = 24.0 * HOUR;

/// A meal: clock time of start (h, may exceed 24 for day 2) and carbohydrate load (g).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meal {
    pub at_h: f64,
    pub carbs_g: f64,
}

/// A GLP-1 agonist dose: clock time (h) and amount (arbitrary dose units, 1.0 = "standard").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dose {
    pub at_h: f64,
    pub units: f64,
}

/// Tunable model parameters. `fit::FIT_KEYS` names the ones fitted to published data.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    // leptin: L0 * (1 + a1 cos(w(h - p1)) + a2 cos(2w(h - p2)))
    pub lep_mean: f64,
    pub lep_a1: f64,
    pub lep_p1: f64,
    pub lep_a2: f64,
    pub lep_p2: f64,
    pub lep_tau_s: f64,
    // ghrelin
    pub ghr_base: f64,
    pub ghr_rise: f64,
    pub ghr_fast_tau_s: f64,
    pub ghr_supp: f64,
    pub ghr_tau_s: f64,
    pub ghr_circ: f64,
    // gut + GLP-1
    pub gastric_k: f64,
    pub gut_k: f64,
    pub glp1_base: f64,
    pub glp1_gain: f64,
    pub glp1_tau_s: f64,
    // glucose + insulin
    pub glu_base: f64,
    pub glu_gain: f64,
    pub glu_k: f64,
    pub ins_base: f64,
    pub ins_gain: f64,
    pub ins_incretin: f64,
    pub ins_tau_s: f64,
    pub ins_glu_effect: f64,
    // sleep loss (Spiegel 2004 contrast is 4 h vs 10 h in bed)
    pub sleep_lep: f64,
    pub sleep_ghr: f64,
    // GLP-1 agonist PK (assumption)
    pub drug_ka: f64,
    pub drug_tau_s: f64,
    pub drug_pmol_per_unit: f64,
    /// agonist slows gastric emptying: k_empt / (1 + drug_empt_slow * plasma units)
    pub drug_empt_slow: f64,
    // PYY (pmol/l): rises with nutrients in the gut, first-order lag
    pub pyy_base: f64,
    pub pyy_gain: f64,
    pub pyy_tau_s: f64,
    // cortisol (ug/dl): circadian + awakening response + stress, lifted by sleep debt
    pub cort_min: f64,
    pub cort_amp: f64,
    pub cort_peak_h: f64,
    pub cort_tau_s: f64,
    pub car_amp: f64,
    pub car_delay_h: f64,
    pub car_width_h: f64,
    pub cort_stress: f64,
    pub cort_debt: f64,
    /// reference level used by the neural coupling (daily mean of the circadian curve)
    pub cort_ref: f64,
    // cumulative sleep debt (0..1) relaxes toward the acute severity with this time constant
    pub debt_tau_s: f64,
    // circadian misalignment lowers insulin action by this fraction at 12 h of misalignment
    pub misalign_isens: f64,
    // illness lowers the ghrelin target by this fraction at illness = 1 (assumption)
    pub ill_ghr: f64,
}

impl Params {
    /// Hand-set starting values used before any fit. `fit.rs` always starts here, so the
    /// leave-one-out check never sees the held-out target through its starting point.
    pub fn prior() -> Self {
        Params {
            lep_mean: 12.0,
            lep_a1: 0.26,
            lep_p1: 1.5,
            lep_a2: 0.06,
            ghr_rise: 0.9,
            ghr_tau_s: 15.0 * 60.0,
            glp1_tau_s: 10.0 * 60.0,
            ..Params::default()
        }
    }
}

impl Default for Params {
    /// Defaults are the fitted values written by `cargo run --bin metrics` (see `fit.rs`),
    /// rounded. Unfitted values are assumptions.
    fn default() -> Self {
        Params {
            lep_mean: 12.74,
            lep_a1: 0.227,
            lep_p1: 21.91,
            lep_a2: 0.171,
            lep_p2: 1.0,
            lep_tau_s: 2.0 * HOUR,
            ghr_base: 500.0,
            ghr_rise: 0.145,
            ghr_fast_tau_s: 3.0 * HOUR,
            ghr_supp: 0.9,
            ghr_tau_s: 1919.0,
            ghr_circ: 0.1,
            gastric_k: 1.0 / (25.0 * 60.0),
            gut_k: 1.0 / (40.0 * 60.0),
            glp1_base: 5.0,
            glp1_gain: 500.0,
            glp1_tau_s: 2431.0,
            glu_base: 90.0,
            glu_gain: 1.0,
            glu_k: 1.0 / (40.0 * 60.0),
            ins_base: 8.0,
            ins_gain: 1.2,
            ins_incretin: 0.08,
            ins_tau_s: 20.0 * 60.0,
            ins_glu_effect: 0.012,
            sleep_lep: 0.18,
            sleep_ghr: 0.28,
            drug_ka: 1.0 / (6.0 * HOUR),
            drug_tau_s: 24.0 * HOUR,
            drug_pmol_per_unit: 20.0,
            drug_empt_slow: 0.9,
            pyy_base: 12.0,
            pyy_gain: 1.2,
            pyy_tau_s: 30.0 * 60.0,
            cort_min: 2.0,
            cort_amp: 12.0,
            cort_peak_h: 8.0,
            cort_tau_s: 20.0 * 60.0,
            car_amp: 4.0,
            car_delay_h: 0.6,
            car_width_h: 0.3,
            cort_stress: 0.6,
            cort_debt: 0.35,
            cort_ref: 7.5,
            debt_tau_s: 36.0 * HOUR,
            misalign_isens: 0.3,
            ill_ghr: 0.35,
        }
    }
}

/// Model state. Units: ghrelin pg/ml, leptin ng/ml, insulin uU/ml, GLP-1 pmol/l, glucose mg/dl,
/// stomach and gut in g carbohydrate, drug depot and plasma in dose units / pmol/l.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub ghrelin: f64,
    pub leptin: f64,
    pub insulin: f64,
    pub glp1: f64,
    pub glucose: f64,
    pub stomach: f64,
    pub gut: f64,
    pub depot: f64,
    pub drug: f64,
    /// seconds since the last meal started (for the anticipatory ghrelin rise)
    pub since_meal: f64,
    /// PYY, pmol/l
    pub pyy: f64,
    /// cortisol, ug/dl
    pub cortisol: f64,
    /// cumulative sleep debt, 0 (none) to 1 (4 h or less in bed, sustained)
    pub debt: f64,
}

impl State {
    pub fn basal(p: &Params) -> Self {
        State {
            ghrelin: p.ghr_base,
            leptin: p.lep_mean,
            insulin: p.ins_base,
            glp1: p.glp1_base,
            glucose: p.glu_base,
            stomach: 0.0,
            gut: 0.0,
            depot: 0.0,
            drug: 0.0,
            since_meal: 4.0 * HOUR,
            pyy: p.pyy_base,
            cortisol: p.cort_ref,
            debt: 0.0,
        }
    }
    pub fn as_array(&self) -> [f64; 13] {
        [
            self.ghrelin,
            self.leptin,
            self.insulin,
            self.glp1,
            self.glucose,
            self.stomach,
            self.gut,
            self.depot,
            self.drug,
            self.since_meal,
            self.pyy,
            self.cortisol,
            self.debt,
        ]
    }
    pub fn is_finite(&self) -> bool {
        self.as_array().iter().all(|v| v.is_finite())
    }
    /// Total GLP-1 receptor signal: endogenous plus agonist.
    pub fn glp1_total(&self, p: &Params) -> f64 {
        self.glp1 + self.drug * p.drug_pmol_per_unit
    }
}

/// Circadian leptin shape (dimensionless, mean ~1) at clock hour `h`.
pub fn leptin_shape(p: &Params, h: f64) -> f64 {
    let w = 2.0 * PI / 24.0;
    1.0 + p.lep_a1 * (w * (h - p.lep_p1)).cos() + p.lep_a2 * (2.0 * w * (h - p.lep_p2)).cos()
}

/// Sleep-loss severity in [0, 1]: 0 at >= 10 h in bed, 1 at <= 4 h (Spiegel 2004 contrast).
pub fn sleep_debt(sleep_h: f64) -> f64 {
    ((10.0 - sleep_h) / 6.0).clamp(0.0, 1.0)
}

/// Daily inputs for a hormone run.
#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    pub meals: Vec<Meal>,
    pub doses: Vec<Dose>,
    pub sleep_h: f64,
}

impl Day {
    pub fn standard() -> Self {
        Day {
            meals: vec![
                Meal {
                    at_h: 8.0,
                    carbs_g: 60.0,
                },
                Meal {
                    at_h: 12.5,
                    carbs_g: 75.0,
                },
                Meal {
                    at_h: 18.5,
                    carbs_g: 90.0,
                },
            ],
            doses: vec![],
            sleep_h: 8.0,
        }
    }
}

/// Extra conditions for one day: circadian displacement, stress and illness.
/// `Ext::default()` is the plain model (no displacement, no stress, no illness).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Ext {
    /// hours the internal clock lags the behavioural clock (travel, shift work); may be negative
    pub circ_shift_h: f64,
    /// psychological or physical stress, 0..1
    pub stress: f64,
    /// illness severity, 0..1
    pub illness: f64,
}

impl Ext {
    pub fn clamped(&self) -> Ext {
        Ext {
            circ_shift_h: self.circ_shift_h.clamp(-12.0, 12.0),
            stress: self.stress.clamp(0.0, 1.0),
            illness: self.illness.clamp(0.0, 1.0),
        }
    }
    /// Misalignment between internal and behavioural clock, 0 (aligned) to 1 (12 h apart).
    pub fn misalignment(&self) -> f64 {
        (self.circ_shift_h.abs() / 12.0).clamp(0.0, 1.0)
    }
}

/// Cortisol target (ug/dl) at clock hour `h`: circadian curve + awakening response + stress.
pub fn cortisol_target(p: &Params, h: f64, sleep_h: f64, debt: f64, e: &Ext) -> f64 {
    let w = 2.0 * PI / 24.0;
    let hc = h.rem_euclid(24.0);
    // circadian part follows the internal clock, in [0, 1], peak at cort_peak_h
    let circ = 0.5 * (1.0 + (w * (hc - e.circ_shift_h - p.cort_peak_h)).cos());
    let circ = circ * circ; // sharper peak, long flat trough
                            // awakening response follows the behavioural clock (wake time = 23:00 + sleep hours)
    let wake = (23.0 + sleep_h).rem_euclid(24.0);
    let mut dh = (hc - wake - p.car_delay_h).rem_euclid(24.0);
    if dh > 12.0 {
        dh -= 24.0;
    }
    let car_on = (sleep_h / 6.0).clamp(0.0, 1.0);
    let car = p.car_amp * car_on * (-0.5 * (dh / p.car_width_h).powi(2)).exp();
    let base = p.cort_min * (1.0 + p.cort_debt * debt) + p.cort_amp * circ + car;
    let up = 1.0 + p.cort_stress * e.stress + 0.5 * p.cort_stress * e.illness;
    base * up
}

/// One Euler step of the hormone model over `dt` seconds at absolute clock hour `h`.
/// Meals and doses are impulses applied by the caller via `eat` and `inject`.
pub fn step(s: &mut State, p: &Params, h: f64, dt: f64, sleep_h: f64) {
    step_ext(s, p, h, dt, sleep_h, &Ext::default());
}

/// Like [`step`], with circadian displacement, stress and illness.
pub fn step_ext(s: &mut State, p: &Params, h: f64, dt: f64, sleep_h: f64, ext: &Ext) {
    let e = ext.clamped();
    let acute = sleep_debt(sleep_h);
    let debt = acute; // acute severity drives leptin and ghrelin (fit to Spiegel 2004)
    let hc = h.rem_euclid(24.0);
    let hcirc = (hc - e.circ_shift_h).rem_euclid(24.0);
    // gut: stomach -> gut -> absorbed. The agonist slows gastric emptying.
    let empt = p.gastric_k * s.stomach / (1.0 + p.drug_empt_slow * s.drug);
    let absorb = p.gut_k * s.gut;
    // leptin: slow relaxation toward circadian target, lowered by sleep loss
    let lep_t = p.lep_mean * leptin_shape(p, hcirc) * (1.0 - p.sleep_lep * debt);
    // ghrelin: anticipatory rise with time since meal, suppressed by nutrients in the gut,
    // in phase with leptin (Cummings 2001), raised by sleep loss, lowered by illness
    let fast = 1.0 - (-s.since_meal / p.ghr_fast_tau_s).exp();
    let circ = 1.0 + p.ghr_circ * (leptin_shape(p, hcirc) - 1.0) / p.lep_a1.max(1e-6);
    let nutr = (s.stomach + s.gut) / 50.0;
    let ghr_t = p.ghr_base
        * circ
        * (1.0 + p.ghr_rise * fast)
        * (1.0 + p.sleep_ghr * debt)
        * (1.0 - p.ill_ghr * e.illness)
        / (1.0 + p.ghr_supp * nutr);
    // GLP-1 secreted as gastric contents reach the gut
    let glp_t = p.glp1_base + p.glp1_gain * empt;
    // PYY follows nutrients in the gut
    let pyy_t = p.pyy_base + p.pyy_gain * s.gut;
    // glucose and insulin (minimal model, incretin effect of total GLP-1 signal);
    // misalignment lowers insulin action (Morris 2015)
    let ins_ex = (s.insulin - p.ins_base).max(0.0);
    let isens = 1.0 - p.misalign_isens * e.misalignment();
    let dglu = p.glu_gain * absorb
        - p.glu_k * (s.glucose - p.glu_base)
        - isens * p.ins_glu_effect * ins_ex * (s.glucose / p.glu_base) / 60.0;
    let glp_sig = s.glp1_total(p) - p.glp1_base;
    let ins_t = p.ins_base
        + p.ins_gain
            * (s.glucose - p.glu_base).max(0.0)
            * (1.0 + p.ins_incretin * glp_sig.max(0.0));
    // cortisol
    let cort_t = cortisol_target(p, hc, sleep_h, s.debt, &e);
    // drug PK
    let rel = p.drug_ka * s.depot;

    s.stomach = (s.stomach - dt * empt).max(0.0);
    s.gut = (s.gut + dt * (empt - absorb)).max(0.0);
    s.leptin += dt * (lep_t - s.leptin) / p.lep_tau_s;
    s.ghrelin += dt * (ghr_t - s.ghrelin) / p.ghr_tau_s;
    s.glp1 += dt * (glp_t - s.glp1) / p.glp1_tau_s;
    s.pyy += dt * (pyy_t - s.pyy) / p.pyy_tau_s;
    s.cortisol += dt * (cort_t - s.cortisol) / p.cort_tau_s;
    s.debt += dt * (acute - s.debt) / p.debt_tau_s;
    s.glucose = (s.glucose + dt * dglu).max(20.0);
    s.insulin += dt * (ins_t - s.insulin) / p.ins_tau_s;
    s.depot = (s.depot - dt * rel).max(0.0);
    s.drug = (s.drug + dt * (rel - s.drug / p.drug_tau_s)).max(0.0);
    s.since_meal += dt;
}

pub fn eat(s: &mut State, m: &Meal) {
    s.stomach += m.carbs_g.max(0.0);
    if m.carbs_g > 0.0 {
        s.since_meal = 0.0;
    }
}

pub fn inject(s: &mut State, d: &Dose) {
    s.depot += d.units.max(0.0);
}

/// Hormone-only trace sampled every `dt` seconds over `hours`, starting at clock 00:00.
/// Day 1 is a warm-up; meals and doses with `at_h` in [0, 24) repeat on each day.
pub struct Trace {
    pub h: Vec<f64>,
    pub s: Vec<State>,
}

pub fn run(p: &Params, day: &Day, hours: f64, dt: f64) -> Trace {
    let mut s = State::basal(p);
    let n = (hours * HOUR / dt).round() as usize;
    let mut tr = Trace {
        h: Vec::with_capacity(n),
        s: Vec::with_capacity(n),
    };
    for k in 0..n {
        let h = k as f64 * dt / HOUR;
        apply_events(&mut s, day, h, dt);
        step(&mut s, p, h, dt, day.sleep_h);
        tr.h.push(h);
        tr.s.push(s);
    }
    tr
}

/// Apply meals and doses whose (daily repeating) time falls in [h, h + dt).
pub fn apply_events(s: &mut State, day: &Day, h: f64, dt: f64) {
    let dh = dt / HOUR;
    let hc = h.rem_euclid(24.0);
    let hit = |at: f64| {
        let a = at.rem_euclid(24.0);
        a >= hc && a < hc + dh
    };
    for m in &day.meals {
        if hit(m.at_h) {
            eat(s, m);
        }
    }
    for d in &day.doses {
        if hit(d.at_h) {
            inject(s, d);
        }
    }
}

/// Map hormone state to neural drive (Hz) for the AgRP and POMC populations.
/// Weights are assumptions; no human AgRP/POMC firing data was available to fit them.
#[derive(Clone, Debug, PartialEq)]
pub struct Coupling {
    pub agrp_rest: f64,
    pub w_ghr: f64,
    pub w_lep: f64,
    pub w_ins: f64,
    pub w_glp: f64,
    pub pomc_rest: f64,
    pub p_lep: f64,
    pub p_glp: f64,
    /// PYY inhibits AgRP and excites POMC (Hz per pmol/l above basal); assumption
    pub w_pyy: f64,
    pub p_pyy: f64,
    /// cortisol excites AgRP (Hz per ug/dl above the reference); assumption
    pub w_cort: f64,
}

impl Default for Coupling {
    fn default() -> Self {
        Coupling {
            agrp_rest: 8.0,
            w_ghr: 58.89,
            w_lep: 6.0,
            w_ins: 0.6,
            w_glp: 0.15,
            pomc_rest: 3.0,
            p_lep: 2.0,
            p_glp: 0.04,
            w_pyy: 0.08,
            p_pyy: 0.02,
            w_cort: 0.25,
        }
    }
}

pub fn neural_drive(s: &State, p: &Params, c: &Coupling) -> [f64; 2] {
    let g = s.ghrelin / p.ghr_base - 1.0;
    let l = s.leptin / p.lep_mean - 1.0;
    let i = s.insulin / p.ins_base - 1.0;
    let glp = s.glp1_total(p) - p.glp1_base;
    let py = s.pyy - p.pyy_base;
    let co = s.cortisol - p.cort_ref;
    let agrp = c.agrp_rest + c.w_ghr * g - c.w_lep * l - c.w_ins * i - c.w_glp * glp - c.w_pyy * py
        + c.w_cort * co;
    let pomc = c.pomc_rest + c.p_lep * l + c.p_glp * glp + c.p_pyy * py;
    [agrp.clamp(0.0, 120.0), pomc.clamp(0.0, 120.0)]
}
