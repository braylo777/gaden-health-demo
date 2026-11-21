# gaden-health-demo

A simulation of the Gaden framework: closed-loop control of the brain's hunger circuit (paper: "Closed-Loop Electrochemical Neural Interfaces for Hypothalamic Appetite Regulation").

> **Simulation only. Not medical advice.** This is a computer model. It is not a medical device. It is not tested in animals or people. It makes no health claim. Do not use it to make decisions about food, sleep, drugs or treatment.

## Results (v0.2.0)

All numbers come from `cargo run --release --bin metrics`, which writes [`metrics.json`](metrics.json).

| Metric | Value |
|---|---|
| AgRP peak, nominal day, closed loop (PID) | **1.04 Hz** (open loop: 30.0 Hz) |
| Stress suite pass rate (best controller, PID) | **82.9%** of 240 scenarios |
| Controllers benchmarked | 3 (PID, MPC, bandit) + open loop |
| Fit to published values, relative RMSE | **0.012** in sample; **0.55** leave-one-out |
| Tests | 178 Rust, all passing (+ 7 Python) |
| Line coverage (library) | 97.1% |
| Rust lines (non-blank) | 3,259 |

### Controller benchmark (240 scenarios each)

| Controller | Pass | Worst AgRP | Mean time AgRP < 2 Hz | Mean AgRP current |
|---|---|---|---|---|
| PID | 82.9% | 17.5 Hz | 95.9% | 15.9 µA |
| MPC | 82.5% | 17.5 Hz | 95.9% | 15.9 µA |
| Bandit (PI gains) | 81.7% | 17.9 Hz | 95.1% | 20.3 µA |
| Open loop | 0.0% | 46.0 Hz | 25.1% | 0 µA |

PID and MPC are almost equal. Both fail in the same scenarios: the AgRP drive is higher than the 40 µA limit can cancel at the plant gain of that scenario. A better algorithm cannot fix that; only a higher current limit or a second actuator can. The bandit explores high-gain arms and overshoots, so it uses more current and passes less.

### Pass rate by scenario type (PID)

| Type | Pass |
|---|---|
| Nominal | 2 / 2 |
| GLP-1 dose (8 h sleep, standard meals) | 10 / 10 |
| Sleep loss (standard meals) | 13 / 18 |
| Fasting (skip breakfast, skip lunch, 24-h fast) | 69 / 90 |
| Binge (big dinner, night binge, grazing, big breakfast) | 105 / 120 |

Run `cargo run --release --example failures` to list every failed scenario.

## Methods

### Hormone model (`rust/src/hormones.rs`)
- **State:** ghrelin, leptin, insulin, GLP-1, glucose, stomach and gut carbohydrate, GLP-1 agonist depot and plasma level.
- **Circadian timing:** leptin follows a 24-h shape with a first and second harmonic. Ghrelin between meals is in phase with leptin.
- **Meals:** carbohydrate moves from stomach to gut to blood. GLP-1 rises as the stomach empties. Insulin rises with glucose, plus an incretin term from GLP-1.
- **Ghrelin:** rises with time since the last meal. Nutrients in the gut suppress it.
- **Sleep loss:** lowers leptin and raises ghrelin. The size comes from the fit (Spiegel 2004).
- **GLP-1 agonist:** first-order absorption and clearance. These rates are assumptions.
- **Neural drive:** ghrelin excites AgRP; leptin, insulin and GLP-1 inhibit it. Leptin and GLP-1 excite POMC. The ghrelin weight is calibrated so that the open-loop nominal day peaks at 30 Hz, the v0.1 scale. The other weights are assumptions.

### Plant and pass rule (`rust/src/sim.rs`)
- The plant is the v0.1 two-population first-order model (τ = 5 s, current limits 40 µA AgRP and 25 µA POMC).
- One run = a 24-h warm-up day (hormones only) + a 24-h closed-loop day at 1 s steps.
- **Pass:** AgRP < 2 Hz for ≥ 95% of the day, POMC mean within 1 Hz of 8 Hz, all states finite and bounded.

### Stress suite
8 meal patterns × 5 sleep levels (10, 8, 6, 4, 0 h) × 6 dose regimes (none, 1×, 2×, start, stop, double) = **240 scenarios**. Each scenario has a seeded plant-gain error from 0.7 to 1.3 per channel and 0.3 Hz sensor noise.

### Controllers (`rust/src/controllers.rs`)
- **PID:** derivative on measurement, conditional-integration anti-windup.
- **MPC:** first-order internal model with a disturbance estimate (offset-free). The input is constant over a 20-step horizon, so the optimum has a closed form. The result is clamped to the limits.
- **Bandit:** epsilon-greedy (ε = 0.1) over 5 PI gain multipliers (0.5× to 2×). It scores each 60-s window by mean squared error. This is a minimal reinforcement-learning baseline, not a deep RL agent.

### Data fit (`rust/src/fit.rs`, `data/`)
- 9 published summary values from 4 PubMed abstracts (Cummings 2001, Sinha 1996, Elliott 1993, Spiegel 2004). The exact quotes and citations are in [`data/SOURCES.md`](data/SOURCES.md).
- Nelder-Mead fits 9 parameters, starting from hand-set prior values. Residuals are relative to each target. Clock times use circular differences.
- Per-target results: [`data/fit_result.json`](data/fit_result.json).

## Limitations — read these

- **Summary values, not time series.** No figure was digitized. The fit uses 9 numbers from abstracts.
- **The fit is exactly determined.** 9 free parameters against 9 targets, so a small in-sample error is expected and proves little. The leave-one-out error (0.55) is the honest number. It shows the model does **not** predict a held-out value well.
- **No AgRP or POMC data.** The coupling from hormones to firing rate is an assumption, calibrated only to the v0.1 30 Hz scale.
- **Unfitted parts:** glucose and insulin kinetics, agonist pharmacokinetics and most time constants are assumptions.
- **Interpretation choices:** the Sinha "amplitude", the meal clock times and the sleep mapping are listed in `data/SOURCES.md`.
- **Toy plant.** First order, one time constant. Real hypothalamic dynamics are not known here.
- **Stability:** tests check the linearised PI loop and bounded-input / bounded-output behaviour by random sampling. This is not a formal proof (the paper's Lyapunov proof is not reproduced).
- Chapter 4 of the paper (electrochemistry, tissue safety) is not modelled.

## Run it

```bash
cd rust
cargo test --release                          # 178 tests
cargo run --release --bin metrics             # writes ../metrics.json (runs tests + coverage)
cargo run --release --bin metrics -- --no-cov # faster; skips cargo-llvm-cov
cargo run --release --example failures        # failed scenarios per controller
```

Python reference (v0.1):

```bash
pip install numpy matplotlib pytest
python -m pytest -q
python -m gaden demo
```

## Layout

| Path | What it is |
|---|---|
| `rust/src/hormones.rs` | Multi-hormone model with circadian timing |
| `rust/src/controllers.rs` | PID, MPC, bandit |
| `rust/src/sim.rs` | 24-h closed loop, 240-scenario suite, calibration |
| `rust/src/fit.rs` | Fit and leave-one-out check |
| `rust/src/bin/metrics.rs` | Writes `metrics.json` |
| `rust/src/{neuron,pomc,control}.rs` | v0.1 Rust port, parity-tested against Python |
| `gaden/` | v0.1 Python reference |
| `data/` | Cited targets, sources, fit result |

See [`CHANGELOG.md`](CHANGELOG.md).
