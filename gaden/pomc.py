"""POMC burst waveform S(t) from the paper (Ch. 2.2).

30 Hz intra-burst, 500 ms bursts every 1500 ms, 150 us charge-balanced biphasic pulses, 25 uA.
"""

import numpy as np

F_BURST_HZ = 30.0
BURST_MS = 500.0
INTER_MS = 1500.0
PULSE_US = 150.0
AMP_UA = 25.0


def burst_envelope(t_ms):
    return (np.asarray(t_ms) % INTER_MS) < BURST_MS


def pulse_times_ms(t_end_ms):
    """Start time of every pulse inside every burst."""
    out, k = [], 0
    while k * INTER_MS < t_end_ms:
        t = k * INTER_MS
        while t < k * INTER_MS + BURST_MS and t < t_end_ms:
            out.append(t)
            t += 1000.0 / F_BURST_HZ
        k += 1
    return np.array(out)


def charge_per_phase_nC():
    """25 uA x 150 us = 3.75 nC per phase. Biphasic and symmetric, so net charge is 0."""
    return AMP_UA * 1e-6 * PULSE_US * 1e-6 * 1e9


def duty_cycle():
    """Fraction of time current flows (both phases)."""
    return (BURST_MS / INTER_MS) * (F_BURST_HZ * 2 * PULSE_US * 1e-6)
