//! Three controllers for the two-population plant: PID, MPC and a bandit-tuned PI.
//! Channel 0 = AgRP (inhibit; plant gain < 0), channel 1 = POMC (excite; plant gain > 0).
//! Output u is a stimulation current in uA, clamped to [0, u_max].

use crate::control::Pi;

pub trait Controller {
    fn name(&self) -> &'static str;
    /// `y` = filtered firing-rate estimate (Hz), `dt` in s. Returns u in [0, u_max].
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2];
    fn reset(&mut self);
}

pub const TARGET: [f64; 2] = [1.0, 8.0];
/// Sign that maps a positive "push toward target" to a positive current.
pub const SIGN: [f64; 2] = [-1.0, 1.0];

// ---------------------------------------------------------------- PID

#[derive(Clone, Debug)]
pub struct Pid {
    pub target: [f64; 2],
    pub kp: [f64; 2],
    pub ki: [f64; 2],
    pub kd: [f64; 2],
    z: [f64; 2],
    y_prev: Option<[f64; 2]>,
}

impl Default for Pid {
    fn default() -> Self {
        Pid {
            target: TARGET,
            kp: [0.3, 0.6],
            ki: [0.15, 0.25],
            kd: [0.2, 0.2],
            z: [0.0; 2],
            y_prev: None,
        }
    }
}

impl Controller for Pid {
    fn name(&self) -> &'static str {
        "pid"
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        let yp = self.y_prev.unwrap_or(y);
        let mut u = [0.0; 2];
        for i in 0..2 {
            let e = self.target[i] - y[i];
            // derivative on measurement (no kick on set-point change)
            let d = -(y[i] - yp[i]) / dt;
            let raw = SIGN[i] * (self.kp[i] * e + self.ki[i] * self.z[i] + self.kd[i] * d);
            u[i] = raw.clamp(0.0, u_max[i]);
            if u[i] == raw {
                self.z[i] += e * dt; // conditional integration anti-windup
            }
        }
        self.y_prev = Some(y);
        u
    }
    fn reset(&mut self) {
        self.z = [0.0; 2];
        self.y_prev = None;
    }
}

// ---------------------------------------------------------------- MPC

/// Offset-free MPC on a first-order internal model x+ = x + dt/tau (d + g u - x).
/// The disturbance d is estimated each step. The input is held constant over the horizon,
/// so the optimum has a closed form; it is then clamped to the limits.
#[derive(Clone, Debug)]
pub struct Mpc {
    pub target: [f64; 2],
    pub tau: f64,
    pub gain: [f64; 2],
    pub horizon: usize,
    pub lambda: [f64; 2],
    pub d_alpha: f64,
    d: [f64; 2],
    prev: Option<([f64; 2], [f64; 2])>,
}

impl Default for Mpc {
    fn default() -> Self {
        Mpc {
            target: TARGET,
            tau: 5.0,
            gain: [-1.0, 0.5],
            horizon: 20,
            lambda: [1e-3, 1e-3],
            d_alpha: 0.3,
            d: [f64::NAN; 2],
            prev: None,
        }
    }
}

impl Mpc {
    /// Closed-form constant-input optimum for one channel, before clamping.
    pub fn solve(&self, i: usize, x0: f64, d: f64, dt: f64) -> f64 {
        let a = (1.0 - dt / self.tau).clamp(0.0, 1.0);
        let g = self.gain[i];
        // x_k = s + (x0 - s) a^k with s = d + g u  =>  x_k - r = c_k + b_k u
        let (mut sbb, mut sbc) = (0.0, 0.0);
        let mut ak = 1.0;
        for _ in 0..self.horizon {
            ak *= a;
            let b = g * (1.0 - ak);
            let c = d * (1.0 - ak) + x0 * ak - self.target[i];
            sbb += b * b;
            sbc += b * c;
        }
        -sbc / (sbb + self.lambda[i])
    }
}

impl Controller for Mpc {
    fn name(&self) -> &'static str {
        "mpc"
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        if let Some((yp, up)) = self.prev {
            for i in 0..2 {
                // invert the model: d = tau (x+ - x)/dt + x - g u
                let est = self.tau * (y[i] - yp[i]) / dt + yp[i] - self.gain[i] * up[i];
                self.d[i] = if self.d[i].is_finite() {
                    self.d[i] + self.d_alpha * (est - self.d[i])
                } else {
                    est
                };
            }
        } else {
            for i in 0..2 {
                self.d[i] = y[i];
            }
        }
        let mut u = [0.0; 2];
        for i in 0..2 {
            let raw = self.solve(i, y[i], self.d[i], dt);
            u[i] = if raw.is_finite() {
                raw.clamp(0.0, u_max[i])
            } else {
                0.0
            };
        }
        self.prev = Some((y, u));
        u
    }
    fn reset(&mut self) {
        self.d = [f64::NAN; 2];
        self.prev = None;
    }
}

// ---------------------------------------------------------------- Bandit

/// Epsilon-greedy bandit over gain multipliers for a PI controller.
/// Every `window` seconds it scores the last window (reward = -mean squared error, both channels)
/// and picks the next arm. It is a minimal RL baseline, not a deep RL agent.
#[derive(Clone, Debug)]
pub struct Bandit {
    pub arms: Vec<f64>,
    pub eps: f64,
    pub window: f64,
    pi: Pi,
    q: Vec<f64>,
    n: Vec<u32>,
    arm: usize,
    acc: f64,
    acc_t: f64,
    rng: u64,
    seed: u64,
}

impl Bandit {
    pub fn new(seed: u64) -> Self {
        let arms = vec![0.5, 0.75, 1.0, 1.5, 2.0];
        let k = arms.len();
        Bandit {
            arms,
            eps: 0.1,
            window: 60.0,
            pi: Pi::default(),
            q: vec![0.0; k],
            n: vec![0; k],
            arm: 2,
            acc: 0.0,
            acc_t: 0.0,
            rng: seed.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1,
            seed,
        }
    }
    pub fn values(&self) -> &[f64] {
        &self.q
    }
    pub fn counts(&self) -> &[u32] {
        &self.n
    }
    pub fn current_arm(&self) -> usize {
        self.arm
    }
    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }
    fn choose(&mut self) -> usize {
        // try every arm once, then epsilon-greedy
        if let Some(i) = self.n.iter().position(|&c| c == 0) {
            return i;
        }
        if self.rand() < self.eps {
            ((self.rand() * self.arms.len() as f64) as usize).min(self.arms.len() - 1)
        } else {
            let mut best = 0;
            for i in 1..self.q.len() {
                if self.q[i] > self.q[best] {
                    best = i;
                }
            }
            best
        }
    }
}

impl Controller for Bandit {
    fn name(&self) -> &'static str {
        "bandit"
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        let e0 = (y[0] - TARGET[0]).max(0.0); // only AgRP above target is a cost
        let e1 = y[1] - TARGET[1];
        self.acc += (e0 * e0 + e1 * e1) * dt;
        self.acc_t += dt;
        if self.acc_t >= self.window {
            let r = -self.acc / self.acc_t;
            let a = self.arm;
            self.n[a] += 1;
            self.q[a] += (r - self.q[a]) / self.n[a] as f64;
            self.acc = 0.0;
            self.acc_t = 0.0;
            self.arm = self.choose();
        }
        let m = self.arms[self.arm];
        let base = Pi::default();
        self.pi.kp = [base.kp[0] * m, base.kp[1] * m];
        self.pi.ki = [base.ki[0] * m, base.ki[1] * m];
        self.pi.step(y, dt, u_max)
    }
    fn reset(&mut self) {
        *self = Bandit::new(self.seed);
    }
}

pub fn by_name(name: &str, seed: u64) -> Option<Box<dyn Controller>> {
    match name {
        "pid" => Some(Box::new(Pid::default())),
        "mpc" => Some(Box::new(Mpc::default())),
        "bandit" => Some(Box::new(Bandit::new(seed))),
        _ => None,
    }
}

pub const NAMES: [&str; 3] = ["pid", "mpc", "bandit"];
