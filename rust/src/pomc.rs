//! POMC burst waveform (port of `gaden/pomc.py`).

pub const F_BURST_HZ: f64 = 30.0;
pub const BURST_MS: f64 = 500.0;
pub const INTER_MS: f64 = 1500.0;
pub const PULSE_US: f64 = 150.0;
pub const AMP_UA: f64 = 25.0;

pub fn burst_envelope(t_ms: f64) -> bool {
    t_ms.rem_euclid(INTER_MS) < BURST_MS
}

pub fn pulse_times_ms(t_end_ms: f64) -> Vec<f64> {
    let mut out = vec![];
    let mut k = 0.0;
    while k * INTER_MS < t_end_ms {
        let mut t = k * INTER_MS;
        while t < k * INTER_MS + BURST_MS && t < t_end_ms {
            out.push(t);
            t += 1000.0 / F_BURST_HZ;
        }
        k += 1.0;
    }
    out
}

/// 25 uA x 150 us = 3.75 nC per phase; biphasic, so net charge is 0.
pub fn charge_per_phase_nc() -> f64 {
    AMP_UA * 1e-6 * PULSE_US * 1e-6 * 1e9
}

pub fn duty_cycle() -> f64 {
    (BURST_MS / INTER_MS) * (F_BURST_HZ * 2.0 * PULSE_US * 1e-6)
}
