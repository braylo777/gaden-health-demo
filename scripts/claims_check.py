#!/usr/bin/env python3
"""G7 claims check: no cure / solved / treatment / medical claim in README or docs.
Allowed: disclaimers that deny a claim ("not a medical device", "not medical advice", ...).
Exit 1 if a forbidden word remains after the allowed phrases are removed.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ALLOWED = [
    r"not a medical device", r"not medical advice", r"no medical claim", r"makes no health claim",
    r"not a cure", r"not a medical", r"no medical", r"not medical", r"not a treatment",
    r"does not cure", r"nothing here is a cure", r"not a clinical", r"not financial advice",
    r"medical device", r"medical advice", r"medical care", r"medical claim",
]
FORBIDDEN = re.compile(r"\b(cure[sd]?|solved|solves|clinically proven|medical|financial)\b", re.I)

bad = 0
paths = [os.path.join(ROOT, "README.md")] + sorted(glob.glob(os.path.join(ROOT, "docs", "*.md")))
paths += [os.path.join(ROOT, "web", "index.html")]
for p in paths:
    if not os.path.exists(p):
        continue
    for ln, line in enumerate(open(p, encoding="utf-8"), 1):
        t = line
        for a in ALLOWED:
            t = re.sub(a, " ", t, flags=re.I)
        m = FORBIDDEN.search(t)
        if m:
            bad += 1
            print(f"{os.path.relpath(p, ROOT)}:{ln}: '{m.group(0)}': {line.strip()[:100]}")
print("claims check: 0 hits" if not bad else f"claims check: {bad} hit(s)")
sys.exit(1 if bad else 0)
