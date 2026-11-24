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

impl Mpc {
    /// Update the disturbance estimate from the last measurement and input.
    fn update_disturbance(&mut self, y: [f64; 2], dt: f64) {
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
            self.d = y;
        }
    }
}

impl Controller for Mpc {
    fn name(&self) -> &'static str {
        "mpc"
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        self.update_disturbance(y, dt);
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

// ---------------------------------------------------------------- Robust MPC

/// Minimax MPC. The plant gain is uncertain by a set of multipliers (default 0.7, 1.0, 1.3,
/// the range of the stress suite). The input is held constant over the horizon and minimises
/// the worst-case squared tracking error over that set. The worst case of convex quadratics is
/// convex in u, so a bounded ternary search finds the optimum.
#[derive(Clone, Debug)]
pub struct RobustMpc {
    pub base: Mpc,
    pub scales: Vec<f64>,
}

impl Default for RobustMpc {
    fn default() -> Self {
        RobustMpc {
            base: Mpc::default(),
            scales: vec![0.7, 1.0, 1.3],
        }
    }
}

impl RobustMpc {
    /// Worst-case cost coefficients for one channel: returns (s_cc, s_bc, s_bb) at unit scale.
    fn sums(&self, i: usize, x0: f64, d: f64, dt: f64) -> (f64, f64, f64) {
        let m = &self.base;
        let a = (1.0 - dt / m.tau).clamp(0.0, 1.0);
        let g = m.gain[i];
        let (mut scc, mut sbc, mut sbb) = (0.0, 0.0, 0.0);
        let mut ak = 1.0;
        for _ in 0..m.horizon {
            ak *= a;
            let b = g * (1.0 - ak);
            let c = d * (1.0 - ak) + x0 * ak - m.target[i];
            scc += c * c;
            sbc += b * c;
            sbb += b * b;
        }
        (scc, sbc, sbb)
    }

    /// Worst-case-optimal input for channel `i`, inside [0, u_hi].
    ///
    /// Each scale k gives a convex quadratic `q_k(u) = scc + 2 k sbc u + (k^2 sbb + lambda) u^2`.
    /// The minimum of their maximum is at the bound, at the minimum of one quadratic, or where two
    /// quadratics cross. The function tries all of these points and keeps the best one.
    pub fn solve(&self, i: usize, x0: f64, d: f64, dt: f64, u_hi: f64) -> f64 {
        let (scc, sbc, sbb) = self.sums(i, x0, d, dt);
        let lam = self.base.lambda[i];
        let hi = u_hi.max(0.0);
        let q: Vec<(f64, f64)> = self
            .scales
            .iter()
            .map(|&k| (k * k * sbb + lam, k * sbc)) // (a_k, b_k): q_k = scc + 2 b_k u + a_k u^2
            .collect();
        let worst = |u: f64| {
            q.iter()
                .map(|&(a, b)| scc + 2.0 * b * u + a * u * u)
                .fold(f64::MIN, f64::max)
        };
        let mut cand = vec![0.0, hi];
        for &(a, b) in &q {
            if a > 0.0 {
                cand.push(-b / a);
            }
        }
        for (m, &(a1, b1)) in q.iter().enumerate() {
            for &(a2, b2) in &q[m + 1..] {
                // q1 - q2 = 0: (a1 - a2) u^2 + 2 (b1 - b2) u = 0
                let (da, db) = (a1 - a2, b1 - b2);
                if da.abs() > 1e-12 {
                    cand.push(-2.0 * db / da);
                } else if db.abs() > 1e-12 {
                    cand.push(0.0);
                }
            }
        }
        let mut best = (f64::MAX, 0.0);
        for u in cand {
            let u = if u.is_finite() { u.clamp(0.0, hi) } else { 0.0 };
            let w = worst(u);
            if w < best.0 {
                best = (w, u);
            }
        }
        best.1
    }
}

impl Controller for RobustMpc {
    fn name(&self) -> &'static str {
        "rmpc"
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        self.base.update_disturbance(y, dt);
        let mut u = [0.0; 2];
        for i in 0..2 {
            let raw = self.solve(i, y[i], self.base.d[i], dt, u_max[i]);
            u[i] = if raw.is_finite() {
                raw.clamp(0.0, u_max[i])
            } else {
                0.0
            };
        }
        self.base.prev = Some((y, u));
        u
    }
    fn reset(&mut self) {
        self.base.reset();
    }
}

// ---------------------------------------------------------------- Safety filter

/// Hard limits on the stimulation current. They hold for any inner controller, including one
/// that returns NaN, negative or huge values. These numbers are design assumptions for this
/// simulation. They are not clinical limits.
#[derive(Clone, Debug, PartialEq)]
pub struct SafetyLimits {
    /// absolute ceiling per channel (uA), applied on top of the device limit `u_max`
    pub peak: [f64; 2],
    /// largest allowed rise per second (uA/s). Falls are not rate-limited.
    pub slew: [f64; 2],
    /// largest allowed running mean (uA), exponential window of `mean_tau_s`
    pub mean_limit: [f64; 2],
    pub mean_tau_s: f64,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        SafetyLimits {
            peak: [38.0, 23.75],
            slew: [10.0, 10.0],
            mean_limit: [28.0, 17.5],
            mean_tau_s: 3600.0,
        }
    }
}

/// Wraps any controller. Invariants after every step, for every input:
/// `0 <= u <= min(u_max, peak)`, `u - u_prev <= slew * dt`, and `mean <= mean_limit`.
pub struct Safe {
    inner: Box<dyn Controller>,
    pub limits: SafetyLimits,
    name: &'static str,
    last: [f64; 2],
    mean: [f64; 2],
}

impl Safe {
    pub fn new(inner: Box<dyn Controller>, limits: SafetyLimits, name: &'static str) -> Self {
        Safe {
            inner,
            limits,
            name,
            last: [0.0; 2],
            mean: [0.0; 2],
        }
    }
    /// Running mean of the output (uA), per channel.
    pub fn mean(&self) -> [f64; 2] {
        self.mean
    }
    /// Pure filter: clamp a raw command. Used by `step` and by the property tests.
    pub fn filter(&mut self, raw: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        let alpha = if self.limits.mean_tau_s > 0.0 && dt.is_finite() && dt > 0.0 {
            1.0 - (-dt / self.limits.mean_tau_s).exp()
        } else {
            1.0
        };
        let dt_ok = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
        let mut u = [0.0; 2];
        for i in 0..2 {
            let mut v = if raw[i].is_finite() { raw[i] } else { 0.0 };
            let hi = u_max[i].min(self.limits.peak[i]).max(0.0);
            v = v.clamp(0.0, hi);
            v = v.min(self.last[i] + self.limits.slew[i] * dt_ok);
            // mean budget: mean + alpha (v - mean) <= limit
            let cap = if alpha > 0.0 {
                self.mean[i] + (self.limits.mean_limit[i] - self.mean[i]) / alpha
            } else {
                f64::MAX
            };
            v = v.min(cap).max(0.0);
            self.mean[i] += alpha * (v - self.mean[i]);
            self.last[i] = v;
            u[i] = v;
        }
        u
    }
}

impl Controller for Safe {
    fn name(&self) -> &'static str {
        self.name
    }
    fn step(&mut self, y: [f64; 2], dt: f64, u_max: [f64; 2]) -> [f64; 2] {
        let raw = self.inner.step(y, dt, u_max);
        self.filter(raw, dt, u_max)
    }
    fn reset(&mut self) {
        self.inner.reset();
        self.last = [0.0; 2];
        self.mean = [0.0; 2];
    }
}

pub fn by_name(name: &str, seed: u64) -> Option<Box<dyn Controller>> {
    match name {
        "pid" => Some(Box::new(Pid::default())),
        "mpc" => Some(Box::new(Mpc::default())),
        "bandit" => Some(Box::new(Bandit::new(seed))),
        "rmpc" => Some(Box::new(RobustMpc::default())),
        "safe_pid" => Some(Box::new(Safe::new(
            Box::new(Pid::default()),
            SafetyLimits::default(),
            "safe_pid",
        ))),
        "safe_rmpc" => Some(Box::new(Safe::new(
            Box::new(RobustMpc::default()),
            SafetyLimits::default(),
            "safe_rmpc",
        ))),
        _ => None,
    }
}

/// The three v0.2 controllers (kept for the 240-scenario suite).
pub const NAMES: [&str; 3] = ["pid", "mpc", "bandit"];
/// Every closed-loop controller in v1.0.
pub const ALL_NAMES: [&str; 6] = ["pid", "mpc", "bandit", "rmpc", "safe_pid", "safe_rmpc"];
