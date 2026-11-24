"""Write rust/tests/fixtures/parity2.json from the Python reference (noise-free runs only).
Run from the repo root: python scripts/gen_parity2.py
"""
import json
import numpy as np
from gaden import neuron, control

NEURON_I = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 20.0, 30.0, 50.0, 80.0]
PEAKS = [10.0, 20.0, 30.0, 40.0]
GAINS = [(0.7, 0.7), (0.7, 1.3), (1.0, 1.0), (1.3, 0.7), (1.3, 1.3), (0.85, 1.15)]

out = {"neuron": [], "control": []}
for i in NEURON_I:
    r = neuron.simulate(i, t_end=60.0)
    out["neuron"].append(
        {
            "i": i,
            "spikes": int(r["spikes"]),
            "blocked": neuron.is_blocked(r, 0.3),
            "v": [float(r["v"][k * 500]) for k in range(12)],
            "h_end": float(r["h"][-1]),
        }
    )
for peak in PEAKS:
    for ge in GAINS:
        for closed in (True, False):
            r = control.simulate(closed=closed, noise=0.0, gain_err=ge, hunger_peak=peak)
            m = control.metrics(r)
            out["control"].append(
                {
                    "peak": peak,
                    "gain_err": list(ge),
                    "closed": closed,
                    "x": [[float(a) for a in r["x"][k * 100]] for k in range(6)],
                    "u": [[float(a) for a in r["u"][k * 100]] for k in range(6)],
                    "metrics": m,
                }
            )
json.dump(out, open("rust/tests/fixtures/parity2.json", "w"))
print(len(out["neuron"]), len(out["control"]))
