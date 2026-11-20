//! Closed-loop PI controller over a two-population plant (port of `gaden/control.py`).
//! Sensor noise is a parameter; with noise = 0 the run is deterministic and matches Python.

#[derive(Clone, Debug)]
pub struct Plant {
    pub tau: f64,
    pub gain: [f64; 2],
    pub pomc_base: f64,
    pub u_max: [f64; 2],
}

impl Default for Plant {
    fn default() -> Self {
        Plant {
            tau: 5.0,
            gain: [-1.0, 0.5],
            pomc_base: 3.0,
            u_max: [40.0, 25.0],
        }
    }
}

impl Plant {
    /// Pre-meal hunger surge: AgRP drive ramps up to `hunger_peak`, then falls after the meal.
    pub fn drive(&self, t: f64, hunger_peak: f64) -> [f64; 2] {
        let (t_peak, t_end, rest) = (300.0, 600.0, 8.0);
        let ramp = if t <= t_peak {
            (t / t_peak).clamp(0.0, 1.0)
        } else {
            (1.0 - (t - t_peak) / (t_end - t_peak)).clamp(0.0, 1.0)
        };
        [rest + (hunger_peak - rest) * ramp, self.pomc_base]
    }
}

#[derive(Clone, Debug)]
pub struct Pi {
    pub target: [f64; 2],
    pub kp: [f64; 2],
    pub ki: [f64; 2],
    pub z: [f64; 2],
}

impl Default for Pi {
    fn default() -> Self {
        Pi {
            target: [1.0, 8.0],
            kp: [0.3, 0.6],
            ki: [0.15, 0.25],
            z: [0.0; 2],
        }
    }
}

impl Pi {
    pub fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        let e = [self.target[0] - y[0], self.target[1] - y[1]];
        let mut raw = [
            self.kp[0] * e[0] + self.ki[0] * self.z[0],
            self.kp[1] * e[1] + self.ki[1] * self.z[1],
        ];
        raw[0] = -raw[0]; // AgRP channel: plant gain is negative
        let mut u = [0.0; 2];
        for i in 0..2 {
            u[i] = raw[i].clamp(0.0, u_max[i]);
            if u[i] == raw[i] {
                self.z[i] += e[i] * dt; // anti-windup
            }
        }
        u
    }
}

/// Real parts of the linearised closed-loop eigenvalues, per channel. All < 0 means stable.
pub fn eigen_real_parts(p: &Plant, c: &Pi) -> Vec<[f64; 2]> {
    (0..2)
        .map(|i| {
            let (g, s) = (p.gain[i], if i == 0 { -1.0 } else { 1.0 });
            let a11 = (-1.0 - g * s * c.kp[i]) / p.tau;
            let a12 = g * s * c.ki[i] / p.tau;
            // [[a11, a12], [-1, 0]]: trace = a11, det = a12
            let (tr, det) = (a11, a12);
            let disc = tr * tr / 4.0 - det;
            if disc >= 0.0 {
                [tr / 2.0 + disc.sqrt(), tr / 2.0 - disc.sqrt()]
            } else {
                [tr / 2.0, tr / 2.0]
            }
        })
        .collect()
}

pub struct Sim {
    pub t: Vec<f64>,
    pub x: Vec<[f64; 2]>,
    pub u: Vec<[f64; 2]>,
}

/// `noise` is the sensor noise standard deviation; pass 0.0 for a deterministic run.
pub fn simulate(closed: bool, hunger_peak: f64, gain_err: [f64; 2], noise: f64, seed: u64) -> Sim {
    let (plant, mut ctrl) = (Plant::default(), Pi::default());
    let (dt, t_end) = (0.1, 600.0);
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut x = plant.drive(0.0, hunger_peak);
    let mut yf = x;
    let n = (t_end / dt) as usize;
    let g = [plant.gain[0] * gain_err[0], plant.gain[1] * gain_err[1]];
    let mut out = Sim {
        t: vec![],
        x: vec![],
        u: vec![],
    };
    for k in 0..n {
        let t = k as f64 * dt;
        let y = [x[0] + noise * rng.normal(), x[1] + noise * rng.normal()];
        let a = (dt / 1.0).min(1.0);
        yf = [yf[0] + (y[0] - yf[0]) * a, yf[1] + (y[1] - yf[1]) * a];
        let u = if closed {
            ctrl.step(yf, dt, plant.u_max)
        } else {
            [0.0; 2]
        };
        let x0 = plant.drive(t, hunger_peak);
        x = [
            x[0] + dt * (x0[0] + g[0] * u[0] - x[0]) / plant.tau,
            x[1] + dt * (x0[1] + g[1] * u[1] - x[1]) / plant.tau,
        ];
        out.t.push(t);
        out.x.push(x);
        out.u.push(u);
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub agrp_max_hz: f64,
    pub agrp_mean_hz: f64,
    pub pomc_mean_hz: f64,
    pub frac_agrp_below_2hz: f64,
    pub u_agrp_peak_ua: f64,
    pub u_pomc_mean_ua: f64,
}

pub fn metrics(s: &Sim, settle: f64) -> Metrics {
    let idx: Vec<usize> = (0..s.t.len()).filter(|&i| s.t[i] >= settle).collect();
    let n = idx.len() as f64;
    Metrics {
        agrp_max_hz: idx.iter().map(|&i| s.x[i][0]).fold(f64::MIN, f64::max),
        agrp_mean_hz: idx.iter().map(|&i| s.x[i][0]).sum::<f64>() / n,
        pomc_mean_hz: idx.iter().map(|&i| s.x[i][1]).sum::<f64>() / n,
        frac_agrp_below_2hz: idx.iter().filter(|&&i| s.x[i][0] < 2.0).count() as f64 / n,
        u_agrp_peak_ua: s.u.iter().map(|u| u[0]).fold(f64::MIN, f64::max),
        u_pomc_mean_ua: s.u.iter().map(|u| u[1]).sum::<f64>() / s.u.len() as f64,
    }
}

/// Small xorshift + Box-Muller generator (no dependencies). Not the numpy stream.
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (u1, u2) = (self.next_f64().max(1e-300), self.next_f64());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Stress test: plant gain error +-30%, hunger peak 20-40 Hz, sensor noise 0.3 Hz.
pub fn monte_carlo(n: usize, seed: u64) -> (f64, f64) {
    let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
    let (mut ok, mut worst) = (0, 0.0f64);
    for i in 0..n {
        let ge = [0.7 + 0.6 * rng.next_f64(), 0.7 + 0.6 * rng.next_f64()];
        let peak = 20.0 + 20.0 * rng.next_f64();
        let m = metrics(&simulate(true, peak, ge, 0.3, i as u64 + 1), 120.0);
        if m.frac_agrp_below_2hz > 0.95 && (m.pomc_mean_hz - 8.0).abs() < 1.0 {
            ok += 1;
        }
        worst = worst.max(m.agrp_max_hz);
    }
    (ok as f64 / n as f64, worst)
}
