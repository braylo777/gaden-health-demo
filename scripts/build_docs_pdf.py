#!/usr/bin/env python3
"""Build docs/HOW-GADEN-WORKS.pdf from the Markdown with headless Chrome. No npm.
  python scripts/build_docs_pdf.py
Needs: python 'markdown' package and Google Chrome (set CHROME to override the path).
"""
import os
import subprocess
import sys
import markdown

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS = os.path.join(ROOT, "docs")
CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
CSS = """
@page { size: A4; margin: 18mm 16mm; }
body { font: 11pt/1.45 Helvetica, Arial, sans-serif; color: #111; }
h1 { font-size: 22pt; margin: 0 0 8pt; } h2 { font-size: 15pt; margin: 18pt 0 6pt; border-bottom: 1px solid #ccc; padding-bottom: 3pt; }
h3 { font-size: 12pt; margin: 12pt 0 4pt; }
blockquote { border-left: 4px solid #b3261e; margin: 8pt 0; padding: 4pt 10pt; background: #fbeeed; }
table { border-collapse: collapse; margin: 8pt 0; font-size: 9.5pt; width: 100%; }
th, td { border: 1px solid #bbb; padding: 3pt 6pt; text-align: left; vertical-align: top; }
th { background: #eef1f5; }
code { font: 9.5pt Menlo, Consolas, monospace; background: #f1f1f1; padding: 0 2pt; }
pre { background: #f6f6f6; padding: 6pt; overflow: hidden; white-space: pre-wrap; font-size: 8.8pt; }
pre code { background: none; }
img { max-width: 100%; page-break-inside: avoid; }
p em { color: #444; font-size: 9.5pt; }
h2, h3 { page-break-after: avoid; } table, img { page-break-inside: avoid; }
"""
src = open(os.path.join(DOCS, "HOW-GADEN-WORKS.md"), encoding="utf-8").read()
body = markdown.markdown(src, extensions=["tables", "fenced_code"])
html = f'<!doctype html><html lang="en"><head><meta charset="utf-8"><title>How Gaden works</title><style>{CSS}</style></head><body>{body}</body></html>'
tmp = os.path.join(DOCS, "_build.html")
open(tmp, "w", encoding="utf-8").write(html)
out = os.path.join(DOCS, "HOW-GADEN-WORKS.pdf")
cmd = [CHROME, "--headless=new", "--disable-gpu", "--no-sandbox", "--allow-file-access-from-files",
       "--no-pdf-header-footer", f"--print-to-pdf={out}", "file://" + tmp]
r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
if not os.path.exists(out):
    print(r.stderr)
    sys.exit(1)
print("wrote", out, os.path.getsize(out), "bytes")
