"""Usage:  python -m gaden demo [--out out]   |   python -m gaden mc [--n 200]"""

import argparse
import json
import os

import numpy as np

from . import control, neuron


def monte_carlo(n=200, seed=1):
    rng = np.random.default_rng(seed)
    ok, worst, means = 0, 0.0, []
    for i in range(n):
        ge, peak = rng.uniform(0.7, 1.3, 2), rng.uniform(20, 40)
        m = control.metrics(
            control.simulate(gain_err=tuple(ge), seed=i, hunger_peak=peak)
        )
        ok += m["frac_agrp_below_2hz"] > 0.95 and abs(m["pomc_mean_hz"] - 8) < 1.0
        worst = max(worst, m["agrp_max_hz"])
        means.append(m["agrp_mean_hz"])
    return {
        "runs": n,
        "pass_rate": ok / n,
        "worst_agrp_max_hz": worst,
        "mean_agrp_hz": float(np.mean(means)),
        "criteria": "after 120 s: AgRP < 2 Hz for >95% of time and POMC mean within 1 Hz of 8 Hz; "
        "plant gain error +-30%, hunger peak 20-40 Hz, sensor noise 0.3 Hz",
    }


def demo(out):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    os.makedirs(out, exist_ok=True)
    report = {}

    cases = [
        ("Rest, 0 uA/cm2", 0.0),
        ("Tonic, 20 uA/cm2", 20.0),
        ("High-current block, 300 uA/cm2", 300.0),
    ]
    fig, ax = plt.subplots(3, 1, figsize=(9, 7), sharex=True)
    for a, (name, i) in zip(ax, cases):
        r = neuron.simulate(i, t_end=300.0)
        blk = neuron.is_blocked(r)
        a.plot(r["t"], r["v"], lw=0.8)
        a.set_ylabel("V (mV)")
        a.set_title(f"{name}: {r['spikes']} spikes, blocked={blk}", fontsize=10)
        report[name] = {
            "spikes": r["spikes"],
            "blocked": blk,
            "v_tail_mean_mv": float(r["v"][int(len(r["v"]) * 0.7) :].mean()),
        }
    ax[-1].set_xlabel("ms")
    fig.suptitle("AgRP neuron model (Hodgkin-Huxley, paper rate constants)")
    fig.tight_layout()
    fig.savefig(f"{out}/fig1_agrp_neuron.png", dpi=130)
    plt.close(fig)

    o, c = control.simulate(closed=False), control.simulate(closed=True)
    fig, ax = plt.subplots(2, 1, figsize=(9, 6), sharex=True)
    ax[0].plot(o["t"], o["x"][:, 0], label="AgRP, no controller", c="tab:red")
    ax[0].plot(c["t"], c["x"][:, 0], label="AgRP, closed loop", c="tab:blue")
    ax[0].plot(c["t"], c["x"][:, 1], label="POMC, closed loop", c="tab:green")
    ax[0].axhline(2, ls="--", c="gray", lw=0.8)
    ax[0].set_ylabel("rate (Hz)")
    ax[0].legend(fontsize=8)
    ax[1].plot(c["t"], c["u"][:, 0], label="u1 AgRP suppression")
    ax[1].plot(c["t"], c["u"][:, 1], label="u2 POMC excitation")
    ax[1].set_ylabel("command (uA)")
    ax[1].set_xlabel("s")
    ax[1].legend(fontsize=8)
    fig.suptitle("Simulated hunger surge: the PI controller holds the setpoints")
    fig.tight_layout()
    fig.savefig(f"{out}/fig2_closed_loop.png", dpi=130)
    plt.close(fig)

    report["no_controller"] = control.metrics(o)
    report["closed_loop"] = control.metrics(c)
    report["eigenvalue_real_parts"] = [
        [float(e.real) for e in ev]
        for ev in control.closed_loop_eigenvalues(control.Plant(), control.PI())
    ]
    report["monte_carlo"] = monte_carlo(200)
    with open(f"{out}/report.json", "w") as f:
        json.dump(report, f, indent=1)
    print(json.dumps(report, indent=1))
    print("wrote", out)


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("cmd", choices=["demo", "mc"])
    p.add_argument("--out", default="out")
    p.add_argument("--n", type=int, default=200)
    a = p.parse_args()
    demo(a.out) if a.cmd == "demo" else print(json.dumps(monte_carlo(a.n), indent=1))
