#!/usr/bin/env python3
"""Run the Vega SVG fixtures through the CLI and compare structurally.

usage: vega.py [cli] [filter-substring] [-v]
"""
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(__file__))
import svgdiff  # noqa: E402

ROOT = os.environ.get("VEGA_DIR", "/home/bjorn/code/plantuml/src/test/resources/vega")
CLI = sys.argv[1] if len(sys.argv) > 1 else "target/debug/plantuml-cli"
FILTER = sys.argv[2] if len(sys.argv) > 2 and not sys.argv[2].startswith("-") else ""
VERBOSE = "-v" in sys.argv

results = {}
for dp, _, fs in sorted(os.walk(ROOT)):
    for f in sorted(fs):
        if not f.endswith(".puml"):
            continue
        p = os.path.join(dp, f)
        ref = p[:-5] + ".svg"
        if not os.path.exists(ref) or FILTER not in p:
            continue
        mt = re.search(r'data-diagram-type="([^"]+)"', open(ref, encoding="utf-8").read())
        top = mt.group(1) if mt else "?"
        src = open(p, encoding="utf-8").read()
        m = re.search(r"^@start", src, re.M)
        if not m:
            continue
        head = src[: m.start()]
        out_fmt = re.search(r"^output:\s*(\S+)", head, re.M)
        if out_fmt and out_fmt.group(1) != "svg":
            continue
        if os.environ.get("TYPE") and os.environ["TYPE"] != top:
            continue
        r = subprocess.run([CLI, "-"], input=src[m.start():].encode(), capture_output=True, timeout=30)
        stats = results.setdefault(top, [0, 0, 0, [0]])
        stats[0] += 1
        if r.returncode != 0:
            stats[2] += 1
            if VERBOSE:
                print("ERROR", os.path.relpath(p, ROOT), r.stderr.decode().strip().splitlines()[-1:])
            continue
        try:
            got = ET.fromstring(r.stdout.decode())
            want = ET.parse(ref).getroot()
        except ET.ParseError as e:
            stats[2] += 1
            continue
        out = []
        svgdiff.walk(want, got, "", 0.002, out)
        close = []
        svgdiff.walk(want, got, "", float(os.environ.get("CLOSE_TOL", "0.7")), close)
        if not close:
            stats[3][0] += 1
        if not out:
            stats[1] += 1
        elif VERBOSE:
            print("DIFF", os.path.relpath(p, ROOT), len(out), out[0])
total = [0, 0, 0]
for top, (n, ok, err, extra) in sorted(results.items()):
    print(f"{top:12} total={n:3} identical={ok:3} close={extra[0]:3} errors={err:3}")
    total[0] += n
    total[1] += ok
    total[2] += err
print(f"{'ALL':12} total={total[0]:3} identical={total[1]:3} errors={total[2]:3}")
