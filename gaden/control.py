"""Closed-loop controller (paper Ch. 3).

State x = [f_AgRP, f_POMC] in Hz. Command u = [AgRP suppression, POMC excitation] in uA.
Toy plant, first order per population: dx/dt = (x0(t) + G*u - x) / tau.
PI tracking with actuator limits and anti-windup. Targets from the paper: AgRP < 2 Hz, POMC ~ 8 Hz.
"""

from dataclasses import dataclass, field

import numpy as np


@dataclass
class Plant:
    tau: float = 5.0  # s, population time constant
    gain: tuple = (-1.0, 0.5)  # Hz per uA: u1 lowers AgRP, u2 raises POMC
    pomc_base: float = 3.0  # Hz
    u_max: tuple = (40.0, 25.0)  # uA limits (POMC limit = paper's 25 uA)

    def drive(self, t, hunger_peak=30.0, t_peak=300.0, t_end=600.0, rest=8.0):
        """Pre-meal hunger surge: AgRP drive ramps from rest to hunger_peak, then falls after the meal."""
        if t <= t_peak:
            ramp = np.clip(t / t_peak, 0, 1)
        else:
            ramp = np.clip(1 - (t - t_peak) / (t_end - t_peak), 0, 1)
        return np.array([rest + (hunger_peak - rest) * ramp, self.pomc_base])


@dataclass
class PI:
    target: tuple = (1.0, 8.0)
    kp: tuple = (0.3, 0.6)
    ki: tuple = (0.15, 0.25)
    z: np.ndarray = field(default_factory=lambda: np.zeros(2))

    def step(self, y, dt, u_max):
        e = np.asarray(self.target) - y
        u_raw = np.asarray(self.kp) * e + np.asarray(self.ki) * self.z
        u_raw[0] = -u_raw[0]  # AgRP channel: plant gain is negative
        u = np.clip(u_raw, 0, u_max)
        for i in (0, 1):  # anti-windup: integrate only when not saturated
            if u[i] == u_raw[i]:
                self.z[i] += e[i] * dt
        return u


def closed_loop_eigenvalues(plant, ctrl):
    """Linearised loop per channel, states [x, z]. All real parts < 0 means stable."""
    out = []
    for i in (0, 1):
        g, s = plant.gain[i], (-1.0 if i == 0 else 1.0)
        kp, ki = ctrl.kp[i], ctrl.ki[i]
        a = np.array(
            [[(-1 - g * s * kp) / plant.tau, g * s * ki / plant.tau], [-1.0, 0.0]]
        )
        out.append(np.linalg.eigvals(a))
    return out


def simulate(
    plant=None,
    ctrl=None,
    closed=True,
    t_end=600.0,
    dt=0.1,
    noise=0.3,
    gain_err=(1.0, 1.0),
    seed=0,
    hunger_peak=30.0,
):
    plant, ctrl = plant or Plant(), ctrl or PI()
    rng = np.random.default_rng(seed)
    x = plant.drive(0.0, hunger_peak=hunger_peak).copy()
    y_f = x.copy()
    n = int(t_end / dt)
    ts = np.arange(n) * dt
    X, U = np.empty((n, 2)), np.empty((n, 2))
    g = np.asarray(plant.gain) * np.asarray(gain_err)
    for k, t in enumerate(ts):
        y = x + rng.normal(0, noise, 2)  # noisy LFP-derived rate estimate
        y_f += (y - y_f) * min(1.0, dt / 1.0)  # 1 s low-pass estimator
        u = ctrl.step(y_f, dt, plant.u_max) if closed else np.zeros(2)
        x = x + dt * (plant.drive(t, hunger_peak=hunger_peak) + g * u - x) / plant.tau
        X[k], U[k] = x, u
    return {"t": ts, "x": X, "u": U}


def metrics(res, settle=120.0):
    m = res["t"] >= settle
    x = res["x"][m]
    return {
        "agrp_max_hz": float(x[:, 0].max()),
        "agrp_mean_hz": float(x[:, 0].mean()),
        "pomc_mean_hz": float(x[:, 1].mean()),
        "frac_agrp_below_2hz": float((x[:, 0] < 2.0).mean()),
        "u_agrp_peak_ua": float(res["u"][:, 0].max()),
        "u_pomc_mean_ua": float(res["u"][:, 1].mean()),
    }
