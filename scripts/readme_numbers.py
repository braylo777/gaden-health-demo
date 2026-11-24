#!/usr/bin/env python3
"""Keep README (and docs) numbers equal to results/metrics.json.

Markers in text files:  <!--n:KEY-->value<!--/n-->

  python scripts/readme_numbers.py --check            # exit 1 if any marker differs from the JSON
  python scripts/readme_numbers.py --write            # rewrite markers from the JSON
  python scripts/readme_numbers.py --compare FILE     # exit 1 if FILE (a fresh run) differs from
                                                      # results/metrics.json by more than 0.5 pp
Files scanned: README.md and docs/*.md
"""
import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
METRICS = os.path.join(ROOT, "results", "metrics.json")
MARK = re.compile(r"<!--n:([a-z0-9_]+)-->(.*?)<!--/n-->", re.S)
CTRL = ["pid", "mpc", "bandit", "rmpc", "safe_pid", "safe_rmpc", "open"]
KINDS = ["shift_work", "illness", "travel", "missed_meals", "sensor_noise"]


def pct(x, d=1):
    return f"{100 * x:.{d}f}%"


def values(j):
    v = j["value"]
    out = {
        "generated_date": j["generated_at"][:10],
        "git_sha": j["git_sha"],
        "machine": j["machine"],
        "scenarios": f"{v['scenarios']:,}",
        "best_controller": v["best_controller"],
        "best_pass": pct(v["best_pass"]),
        "best_safe_controller": v["best_safe_controller"],
        "best_safe_pass": pct(v["best_safe_pass"]),
        "open_pass": pct(v["open_pass"]),
        "tests": str(v["tests"]),
        "coverage": pct(v["coverage"]) if v["coverage"] is not None else "n/a",
        "loc_rust": f"{v['loc_rust']:,}",
        "agrp_peak": f"{v['agrp_peak_hz']:.2f} Hz",
        "agrp_peak_open": f"{v['agrp_peak_open_loop_hz']:.1f} Hz",
        "ms_per_day": f"{v['native_ms_per_sim_day']:.1f} ms",
        "fit_rmse": f"{v['fit_rmse']:.3f}",
        "fit_loo": f"{v['fit_loo_rmse']:.2f}",
        "gastric_ratio": f"{v['anchors']['gastric_first_hour_ratio']:.2f}",
        "misalign_ratio": f"{v['anchors']['misalign_glucose_ratio']:.3f}",
        "car_min": f"{v['anchors']['cortisol_car_peak_min']:.0f}",
    }
    for c in CTRL:
        r = v["controllers"][c]
        out[f"pass_{c}"] = pct(r["pass"])
        out[f"ci_{c}"] = f"{100 * r['ci95'][0]:.1f} to {100 * r['ci95'][1]:.1f}%"
        out[f"worst_{c}"] = f"{r['worst_agrp_hz']:.1f} Hz"
        out[f"u_{c}"] = f"{r['mean_u_agrp_ua']:.1f} uA"
        for k in KINDS:
            kk = r["by_kind"][k]
            out[f"kind_{k}_{c}"] = f"{kk['passed']}/{kk['n']}"
    for k in KINDS:
        out[f"n_{k}"] = str(v["scenarios_by_kind"][k])
    return out


def load():
    with open(METRICS) as f:
        return json.load(f)


def files():
    return [os.path.join(ROOT, "README.md")] + sorted(glob.glob(os.path.join(ROOT, "docs", "*.md")))


def main(argv):
    if "--compare" in argv:
        other = json.load(open(argv[argv.index("--compare") + 1]))
        a, b = load()["value"], other["value"]
        bad = []
        for c in CTRL:
            ra, rb = a["controllers"][c]["pass"], b["controllers"][c]["pass"]
            if abs(ra - rb) > 0.005:
                bad.append(f"{c}: committed {ra:.4f} vs fresh {rb:.4f}")
        if a["scenarios"] != b["scenarios"]:
            bad.append("scenario count differs")
        print("\n".join(bad) if bad else "fresh run reproduces results/metrics.json (within 0.5 pp)")
        return 1 if bad else 0
    vals = values(load())
    write = "--write" in argv
    bad, n = [], 0
    for path in files():
        if not os.path.exists(path):
            continue
        text = open(path, encoding="utf-8").read()

        def sub(m):
            nonlocal n
            n += 1
            key = m.group(1)
            if key not in vals:
                bad.append(f"{os.path.basename(path)}: unknown key {key}")
                return m.group(0)
            if m.group(2) != vals[key]:
                bad.append(f"{os.path.basename(path)}: {key} is '{m.group(2)}', JSON says '{vals[key]}'")
            return f"<!--n:{key}-->{vals[key]}<!--/n-->"

        new = MARK.sub(sub, text)
        if write and new != text:
            open(path, "w", encoding="utf-8").write(new)
    if write:
        print(f"wrote {n} markers")
        return 0
    print("\n".join(bad) if bad else f"ok: {n} markers match results/metrics.json")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
