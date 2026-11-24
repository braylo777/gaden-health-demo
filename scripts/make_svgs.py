#!/usr/bin/env python3
"""Build the data figures in docs/img/ from results/figs.json and results/metrics.json.
Figures 01 (gut-brain axis) and 03 (closed loop) are drawn by hand and not touched here.
  python scripts/make_svgs.py
Standard library only. Colors are fixed and checked on white.
"""
import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IMG = os.path.join(ROOT, "docs", "img")
F = json.load(open(os.path.join(ROOT, "results", "figs.json")))
M = json.load(open(os.path.join(ROOT, "results", "metrics.json")))["value"]
FONT = 'font-family="Helvetica, Arial, sans-serif"'
C = {"ghrelin": "#b45f06", "leptin": "#6a1b9a", "insulin": "#1d4e9e", "glp1": "#2e7d32",
     "pyy": "#00838f", "cortisol": "#b3261e", "glucose": "#555555"}


def head(w, h, title, desc, tid):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" '
            f'aria-labelledby="t{tid} d{tid}" {FONT}>\n  <title id="t{tid}">{title}</title>\n  <desc id="d{tid}">{desc}</desc>\n'
            f'  <rect width="{w}" height="{h}" fill="#ffffff"/>\n')


def poly(xs, ys, x0, x1, y0, y1, px, py, pw, ph, color, width=1.8, dash=""):
    pts = []
    for x, y in zip(xs, ys):
        X = px + (x - x0) / (x1 - x0) * pw
        Y = py + ph - (min(max(y, y0), y1) - y0) / (y1 - y0) * ph
        pts.append(f"{X:.1f},{Y:.1f}")
    d = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<polyline points="{" ".join(pts)}" fill="none" stroke="{color}" stroke-width="{width}"{d}/>'


def axes(px, py, pw, ph, x0, x1, y0, y1, xticks, yticks, ylabel, xlabel=""):
    o = [f'<rect x="{px}" y="{py}" width="{pw}" height="{ph}" fill="none" stroke="#999"/>']
    for t in xticks:
        X = px + (t - x0) / (x1 - x0) * pw
        o.append(f'<line x1="{X:.1f}" y1="{py}" x2="{X:.1f}" y2="{py + ph}" stroke="#e3e3e3"/>')
        o.append(f'<text x="{X:.1f}" y="{py + ph + 16}" font-size="11" fill="#333" text-anchor="middle">{t:g}</text>')
    for t in yticks:
        Y = py + ph - (t - y0) / (y1 - y0) * ph
        o.append(f'<line x1="{px}" y1="{Y:.1f}" x2="{px + pw}" y2="{Y:.1f}" stroke="#e3e3e3"/>')
        o.append(f'<text x="{px - 6}" y="{Y + 4:.1f}" font-size="11" fill="#333" text-anchor="end">{t:g}</text>')
    o.append(f'<text x="{px - 40}" y="{py + ph / 2}" font-size="12" fill="#222" text-anchor="middle" transform="rotate(-90 {px - 40} {py + ph / 2})">{ylabel}</text>')
    if xlabel:
        o.append(f'<text x="{px + pw / 2}" y="{py + ph + 34}" font-size="12" fill="#222" text-anchor="middle">{xlabel}</text>')
    return "\n  ".join(o)


# ---- 02: 24 h hormone timeline (nominal day, open loop) -------------------------------------
def fig2():
    s = F["nominal_open"]
    t = s["t_h"]
    w, h = 900, 520
    out = head(w, h, "24-hour hormone timeline", "Seven hormone curves over one simulated day with three meals at 08:00, 12:30 and 18:30. Each curve is scaled to its own minimum and maximum.", 2)
    out += f'  <text x="450" y="26" text-anchor="middle" font-size="17" font-weight="700" fill="#111">One simulated day, standard meals (model output)</text>\n'
    px, py, pw, ph = 70, 50, 800, 300
    out += "  " + axes(px, py, pw, ph, 0, 24, 0, 1, [0, 4, 8, 12, 16, 20, 24], [0, 0.5, 1], "scaled 0 to 1", "clock hour") + "\n"
    for hm in (8.0, 12.5, 18.5):
        X = px + hm / 24 * pw
        out += f'  <line x1="{X:.1f}" y1="{py}" x2="{X:.1f}" y2="{py + ph}" stroke="#444" stroke-dasharray="4 3"/>\n'
        out += f'  <text x="{X:.1f}" y="{py - 6}" font-size="11" fill="#222" text-anchor="middle">meal</text>\n'
    lx, ly = 70, 410
    for i, k in enumerate(["ghrelin", "leptin", "insulin", "glp1", "pyy", "cortisol", "glucose"]):
        v = s[k]
        lo, hi = min(v), max(v)
        sc = [(a - lo) / (hi - lo) if hi > lo else 0 for a in v]
        out += "  " + poly(t, sc, 0, 24, 0, 1, px, py, pw, ph, C[k]) + "\n"
        name = {"glp1": "GLP-1", "pyy": "PYY"}.get(k, k)
        x = lx + (i % 4) * 205
        y = ly + (i // 4) * 44
        out += f'  <rect x="{x}" y="{y - 10}" width="22" height="4" fill="{C[k]}"/><text x="{x + 30}" y="{y - 3}" font-size="12" fill="#111">{name}</text>\n'
        out += f'  <text x="{x + 30}" y="{y + 12}" font-size="11" fill="#555">{lo:.3g} to {hi:.3g}</text>\n'
    out += f'  <text x="450" y="{h - 18}" text-anchor="middle" font-size="11" fill="#444">Units: ghrelin pg/ml, leptin ng/ml, insulin uU/ml, GLP-1 and PYY pmol/l, cortisol ug/dl, glucose mg/dl. Simulation, not data.</text>\n</svg>\n'
    open(os.path.join(IMG, "02-hormone-timeline-24h.svg"), "w").write(out)


# ---- 04: controller comparison with 95% CI ------------------------------------------------
def fig4():
    order = ["open", "safe_rmpc", "safe_pid", "rmpc", "mpc", "bandit", "pid"]
    names = {"open": "Open loop (no control)", "safe_rmpc": "Robust MPC + safety filter", "safe_pid": "PID + safety filter",
             "rmpc": "Robust MPC", "mpc": "MPC", "bandit": "Bandit-tuned PI", "pid": "PID"}
    col = {"open": "#777777", "safe_rmpc": "#b3261e", "safe_pid": "#b3261e", "rmpc": "#1d4e9e", "mpc": "#1d4e9e", "bandit": "#1d4e9e", "pid": "#1d4e9e"}
    w, h = 900, 440
    n = M["scenarios"]
    out = head(w, h, "Controller comparison", f"Pass rate with 95 percent Wilson interval for each controller over {n} scenarios.", 4)
    out += f'  <text x="450" y="26" text-anchor="middle" font-size="17" font-weight="700" fill="#111">Pass rate over {n:,} seeded scenarios, 95% interval</text>\n'
    px, py, pw, ph = 250, 55, 600, 300
    out += f'  <rect x="{px}" y="{py}" width="{pw}" height="{ph}" fill="none" stroke="#999"/>\n'
    for t in (0, 20, 40, 60, 80, 100):
        X = px + t / 100 * pw
        out += f'  <line x1="{X}" y1="{py}" x2="{X}" y2="{py + ph}" stroke="#e3e3e3"/><text x="{X}" y="{py + ph + 16}" font-size="11" fill="#333" text-anchor="middle">{t}%</text>\n'
    bh = ph / len(order)
    for i, k in enumerate(order):
        r = M["controllers"][k]
        y = py + i * bh + bh / 2
        X = px + r["pass"] * pw
        lo, hi = px + r["ci95"][0] * pw, px + r["ci95"][1] * pw
        out += f'  <text x="{px - 10}" y="{y + 4}" font-size="12" fill="#111" text-anchor="end">{names[k]}</text>\n'
        out += f'  <rect x="{px}" y="{y - 11}" width="{max(X - px, 1):.1f}" height="22" fill="{col[k]}" opacity="0.85"/>\n'
        out += f'  <line x1="{lo:.1f}" y1="{y}" x2="{hi:.1f}" y2="{y}" stroke="#111" stroke-width="2"/><line x1="{lo:.1f}" y1="{y - 6}" x2="{lo:.1f}" y2="{y + 6}" stroke="#111" stroke-width="2"/><line x1="{hi:.1f}" y1="{y - 6}" x2="{hi:.1f}" y2="{y + 6}" stroke="#111" stroke-width="2"/>\n'
        out += f'  <text x="{max(hi, X) + 8:.1f}" y="{y + 4}" font-size="12" fill="#111">{100 * r["pass"]:.1f}%</text>\n'
    out += f'  <text x="450" y="{h - 40}" text-anchor="middle" font-size="12" fill="#222">Blue: plain controllers. Red: with the hard safety limits (peak, rise rate, running mean). Grey: no control.</text>\n'
    out += f'  <text x="450" y="{h - 20}" text-anchor="middle" font-size="11" fill="#444">Pass rule: AgRP below 2 Hz for 95% of the day and POMC mean within 1 Hz of 8 Hz. Simulation only. Run {M["scenario_seed"]} seed.</text>\n</svg>\n'
    open(os.path.join(IMG, "04-controller-comparison.svg"), "w").write(out)


# ---- 05: failure gallery ------------------------------------------------------------------
def fig5():
    fails = F["failures"]
    w, h = 900, 560
    out = head(w, h, "Failure gallery", "Four failed days of the PID controller, one per scenario kind. The AgRP rate leaves the band below 2 Hz when the drive is larger than the current limit can cancel.", 5)
    out += '  <text x="450" y="26" text-anchor="middle" font-size="17" font-weight="700" fill="#111">Failure gallery: worst failed PID day of four scenario kinds</text>\n'
    label = {"shift_work": "Shift work", "travel": "Travel (time-zone shift)", "missed_meals": "Missed meals", "sensor_noise": "Sensor noise"}
    for i, f in enumerate(fails):
        col, row = i % 2, i // 2
        px, py, pw, ph = 70 + col * 440, 70 + row * 240, 360, 150
        s = f["series"]
        out += f'  <text x="{px}" y="{py - 10}" font-size="13" font-weight="700" fill="#111">{label[f["kind"]]}  ({f["name"]})</text>\n'
        ymax = 20
        out += "  " + axes(px, py, pw, ph, 0, 24, 0, ymax, [0, 6, 12, 18, 24], [0, 2, 10, 20], "AgRP rate (Hz)", "clock hour of the day") + "\n"
        yb = py + ph - 2 / ymax * ph
        out += f'  <rect x="{px}" y="{yb:.1f}" width="{pw}" height="{py + ph - yb:.1f}" fill="#2e7d32" opacity="0.15"/>\n'
        out += "  " + poly(s["t_h"], s["drive_agrp"], 0, 24, 0, ymax, px, py, pw, ph, "#999999", 1.3, "4 3") + "\n"
        out += "  " + poly(s["t_h"], s["agrp"], 0, 24, 0, ymax, px, py, pw, ph, "#b3261e", 1.8) + "\n"
        out += f'  <text x="{px + pw}" y="{py - 10}" font-size="11" fill="#333" text-anchor="end">{100 * f["frac_below_2hz"]:.0f}% of day below 2 Hz</text>\n'
    out += '  <rect x="70" y="522" width="22" height="4" fill="#b3261e"/><text x="98" y="527" font-size="12" fill="#111">AgRP rate with PID</text>\n'
    out += '  <rect x="270" y="522" width="22" height="3" fill="#999999"/><text x="298" y="527" font-size="12" fill="#111">drive without control</text>\n'
    out += '  <rect x="470" y="518" width="22" height="10" fill="#2e7d32" opacity="0.3"/><text x="498" y="527" font-size="12" fill="#111">target band, below 2 Hz</text>\n'
    out += '</svg>\n'
    open(os.path.join(IMG, "05-failure-gallery.svg"), "w").write(out)


if __name__ == "__main__":
    os.makedirs(IMG, exist_ok=True)
    fig2()
    fig4()
    fig5()
    print("wrote 3 figures")
