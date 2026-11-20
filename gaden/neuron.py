"""Single-neuron Hodgkin-Huxley model, rate constants as written in the Gaden paper (Ch. 2.1).

Units: V in mV, time in ms, current density in uA/cm^2, Cm = 1.0 uF/cm^2.
The paper does not give conductances or reversal potentials; we use the classic squid-axon values.
"""

import numpy as np

G_NA, G_K, G_L = 120.0, 36.0, 0.3
E_NA, E_K, E_L = 50.0, -77.0, -54.4
CM = 1.0


def rates(v):
    am = 0.1 * (35 + v) / (1 - np.exp(-(35 + v) / 10))
    bm = 4.0 * np.exp(-(60 + v) / 18)
    ah = 0.07 * np.exp(-(60 + v) / 20)
    bh = 1.0 / (np.exp(-(30 + v) / 10) + 1)
    an = 0.01 * (50 + v) / (1 - np.exp(-(50 + v) / 10))
    bn = 0.125 * np.exp(-(60 + v) / 80)
    return am, bm, ah, bh, an, bn


def rest_state(v0=-65.0):
    am, bm, ah, bh, an, bn = rates(v0)
    return v0, am / (am + bm), ah / (ah + bh), an / (an + bn)


def simulate(i_stim, t_end=300.0, dt=0.01):
    """i_stim: float (DC) or callable t_ms -> uA/cm^2. Returns t, v, h and the spike count."""
    v, m, h, n = rest_state()
    steps = int(t_end / dt)
    vs, hs = np.empty(steps), np.empty(steps)
    spikes, prev = 0, v
    for k in range(steps):
        i = i_stim(k * dt) if callable(i_stim) else i_stim
        am, bm, ah, bh, an, bn = rates(v)
        m += dt * (am * (1 - m) - bm * m)
        h += dt * (ah * (1 - h) - bh * h)
        n += dt * (an * (1 - n) - bn * n)
        v += (
            dt
            * (
                -G_NA * m**3 * h * (v - E_NA)
                - G_K * n**4 * (v - E_K)
                - G_L * (v - E_L)
                + i
            )
            / CM
        )
        if prev < 0 <= v:
            spikes += 1
        prev = v
        vs[k], hs[k] = v, h
    return {"t": np.arange(steps) * dt, "v": vs, "h": hs, "spikes": spikes}


def is_blocked(res, window_frac=0.3):
    """Depolarization block: in the last window, no spike peaks and V stays above -50 mV."""
    tail = res["v"][int(len(res["v"]) * (1 - window_frac)) :]
    return bool(tail.max() < 0 and tail.mean() > -50.0)
