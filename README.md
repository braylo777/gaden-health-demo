# gaden-health-demo

A working simulation of the Gaden framework: closed-loop control of the brain's hunger circuit (paper: "Closed-Loop Electrochemical Neural Interfaces for Hypothalamic Appetite Regulation").

> **Research and education only.** This is a computer model. It is not a medical device, it is not tested in animals or people, and it makes no health claim.

## What it runs

| Part | Paper section | What the code does |
|---|---|---|
| `gaden/neuron.py` | Ch. 2.1 | Hodgkin-Huxley neuron with the paper's rate constants. Shows rest, firing, and depolarization block at high current. |
| `gaden/pomc.py` | Ch. 2.2 | The POMC burst waveform: 30 Hz, 500 ms bursts every 1.5 s, 150 µs biphasic pulses, 25 µA. |
| `gaden/control.py` | Ch. 3 | Two-population state model and a PI controller with actuator limits and anti-windup. Targets: AgRP < 2 Hz, POMC ≈ 8 Hz. |
| `python -m gaden demo` | — | Makes 2 charts and `out/report.json`, plus a 200-run stress test. |

## Run it

```bash
pip install numpy matplotlib pytest
python -m pytest -q          # 7 tests
python -m gaden demo         # charts + report in ./out
```

## Results (v0.1.0, seed fixed)

| Check | Result |
|---|---|
| Neuron at rest (0 µA/cm²) | 0 spikes |
| Neuron at 20 µA/cm² | 25 spikes in 300 ms |
| Neuron at 300 µA/cm² | blocked: 1 spike, then held at about −34 mV (paper predicts about −35 mV) |
| Hunger surge, no controller | AgRP peaks at 29.7 Hz |
| Hunger surge, closed loop | AgRP max 1.5 Hz, below 2 Hz 100% of the time; POMC mean 8.0 Hz |
| Linear stability | all eigenvalues have real part −0.13 (stable) |
| Stress test (200 runs: ±30% model error, hunger peak 20–40 Hz, sensor noise) | **92% pass.** Failures: the 40 µA limit is too low when hunger is high and the model gain is low (worst AgRP 10.4 Hz). |

## Honest gaps

- The neuron uses squid-axon conductances. The paper gives no AgRP-specific values.
- The two-population plant is a toy (first order, made-up time constant and gains). Real hypothalamic dynamics are unknown here.
- The paper's Lyapunov proof (Ch. 3.2) is not reproduced; the demo checks stability of the linear loop only.
- Chapter 4 (electrochemistry, safety) is not modeled. A real implant needs tissue-safety charge limits, regulatory approval and clinical trials.

## Next steps

1. Fit the plant to published AgRP/POMC firing data.
2. Add a Kalman filter for the LFP-based state estimate.
3. Model charge-density safety limits (Shannon criterion).
