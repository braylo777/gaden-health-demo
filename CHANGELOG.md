# Changelog

## 0.2.0 — 2026-11-21

### Added
- `rust/src/hormones.rs`: ghrelin, leptin, insulin, GLP-1 and glucose model. It has circadian timing, meals, sleep loss and a GLP-1 agonist dose.
- `rust/src/controllers.rs`: PID (derivative on measurement, anti-windup), offset-free MPC (closed form, constant input over the horizon), and an epsilon-greedy bandit that tunes PI gains.
- `rust/src/sim.rs`: 24-h closed loop. Stress suite of 240 scenarios (8 meal patterns x 5 sleep levels x 6 dose regimes). Each scenario has ±30% plant-gain error and 0.3 Hz sensor noise.
- `rust/src/fit.rs`: Nelder-Mead fit of 9 parameters to 9 published values (`data/targets.csv`). Leave-one-out check included.
- `data/SOURCES.md`: 4 PubMed abstracts, with exact quotes and interpretation choices.
- `cargo run --release --bin metrics`: writes `metrics.json` and `data/fit_result.json`.
- `rust/examples/failures.rs`: lists the failed scenarios for each controller.
- 173 new Rust tests (178 total), including property tests for stability and boundedness. CI with fmt, clippy, tests and a 90% line-coverage gate.

### Changed
- The ghrelin-to-AgRP weight is calibrated so that the open-loop nominal day peaks at 30 Hz. This keeps the v0.1 scale.
- The `control::Rng` generator is now public, for reuse by the scenario suite.

### Not changed
- The v0.1 Python reference and the Rust parity tests are kept as they were.

## 0.1.0
- Python simulation (neuron, POMC waveform, PI controller) and Rust port with parity tests.
