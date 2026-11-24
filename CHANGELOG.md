# Changelog

## 1.0.0 - 2026-11-24

### Added
- Model terms: PYY, cortisol (circadian and awakening response), cumulative sleep debt, agonist slowing of gastric emptying, circadian misalignment, stress and illness. Sources in `data/SOURCES.md`.
- Robust MPC (exact minimax over gain 0.7 to 1.3) and a safety filter with hard limits on peak, rise rate and running mean.
- 1,000 seeded scenarios (shift work, illness, travel, missed meals, sensor noise) with Wilson 95% intervals. `results/metrics.json` carries date, git sha and machine.
- 496 tests: per-term bound and literature tests, 10,000-trial safety property tests, golden curves, 64 Python parity cases.
- Paper `docs/HOW-GADEN-WORKS.md` and PDF, 5 SVG figures, browser demo (WebAssembly), Dockerfile, CLI `gaden`.
- CI: ubuntu and macOS, coverage gate, README-numbers check, claims check, reproduction check, wasm and docker jobs.

### Changed
- Calibrated `w_ghr` re-derived with the new terms. README numbers are generated from the JSON.

### Known gap
- The model gives higher postprandial insulin with the agonist. A 2018 trial found lower insulin.

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
