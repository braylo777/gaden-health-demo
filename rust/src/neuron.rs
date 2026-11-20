//! Hodgkin-Huxley neuron with the paper's rate constants (port of `gaden/neuron.py`).

pub const G_NA: f64 = 120.0;
pub const G_K: f64 = 36.0;
pub const G_L: f64 = 0.3;
pub const E_NA: f64 = 50.0;
pub const E_K: f64 = -77.0;
pub const E_L: f64 = -54.4;
pub const CM: f64 = 1.0;

pub fn rates(v: f64) -> [f64; 6] {
    let am = 0.1 * (35.0 + v) / (1.0 - (-(35.0 + v) / 10.0).exp());
    let bm = 4.0 * (-(60.0 + v) / 18.0).exp();
    let ah = 0.07 * (-(60.0 + v) / 20.0).exp();
    let bh = 1.0 / ((-(30.0 + v) / 10.0).exp() + 1.0);
    let an = 0.01 * (50.0 + v) / (1.0 - (-(50.0 + v) / 10.0).exp());
    let bn = 0.125 * (-(60.0 + v) / 80.0).exp();
    [am, bm, ah, bh, an, bn]
}

pub fn rest_state(v0: f64) -> (f64, f64, f64, f64) {
    let [am, bm, ah, bh, an, bn] = rates(v0);
    (v0, am / (am + bm), ah / (ah + bh), an / (an + bn))
}

pub struct Run {
    pub t: Vec<f64>,
    pub v: Vec<f64>,
    pub h: Vec<f64>,
    pub spikes: u32,
}

/// Forward Euler, same update order as the Python version.
pub fn simulate(i_stim: impl Fn(f64) -> f64, t_end: f64, dt: f64) -> Run {
    let (mut v, mut m, mut h, mut n) = rest_state(-65.0);
    let steps = (t_end / dt) as usize;
    let (mut vs, mut hs, mut ts) = (
        Vec::with_capacity(steps),
        Vec::with_capacity(steps),
        Vec::with_capacity(steps),
    );
    let (mut spikes, mut prev) = (0, v);
    for k in 0..steps {
        let t = k as f64 * dt;
        let i = i_stim(t);
        let [am, bm, ah, bh, an, bn] = rates(v);
        m += dt * (am * (1.0 - m) - bm * m);
        h += dt * (ah * (1.0 - h) - bh * h);
        n += dt * (an * (1.0 - n) - bn * n);
        v += dt
            * (-G_NA * m.powi(3) * h * (v - E_NA) - G_K * n.powi(4) * (v - E_K) - G_L * (v - E_L)
                + i)
            / CM;
        if prev < 0.0 && 0.0 <= v {
            spikes += 1;
        }
        prev = v;
        vs.push(v);
        hs.push(h);
        ts.push(t);
    }
    Run {
        t: ts,
        v: vs,
        h: hs,
        spikes,
    }
}

pub fn simulate_dc(i: f64, t_end: f64) -> Run {
    simulate(move |_| i, t_end, 0.01)
}

/// Depolarization block: in the last window, no spike peaks and V stays above -50 mV.
pub fn is_blocked(r: &Run, window_frac: f64) -> bool {
    let n = r.v.len();
    let tail = &r.v[(n as f64 * (1.0 - window_frac)) as usize..];
    let max = tail.iter().cloned().fold(f64::MIN, f64::max);
    let mean = tail.iter().sum::<f64>() / tail.len() as f64;
    max < 0.0 && mean > -50.0
}
