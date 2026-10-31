#!/usr/bin/env python3
"""Writes site/examples.json: every fixture and, when available, every Vega corpus file, for the playground.

Usage: build-examples.py [OUT]     (default site/examples.json)
The Vega corpus is read from $VEGA_DIR, or ../plantuml/src/test/resources/vega when that exists.
"""
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "site", "examples.json")
FRONT_MATTER = re.compile(r"\A---\n.*?\n---\n", re.S)


def collect(base, group):
    found = []
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames.sort()
        for name in sorted(filenames):
            if not name.endswith(".puml"):
                continue
            path = os.path.join(dirpath, name)
            with open(path, encoding="utf-8", errors="replace") as f:
                source = FRONT_MATTER.sub("", f.read(), count=1).strip("\n") + "\n"
            rel = os.path.relpath(path, base)
            directory = os.path.dirname(rel)
            found.append({
                "g": group + (("/" + directory) if directory else ""),
                "n": os.path.splitext(os.path.basename(rel))[0],
                "s": source,
            })
    return found


examples = collect(os.path.join(ROOT, "fixtures"), "fixtures")
vega = os.environ.get("VEGA_DIR") or os.path.join(ROOT, "..", "plantuml", "src", "test", "resources", "vega")
if os.path.isdir(vega):
    examples += collect(vega, "vega")
else:
    print(f"no Vega corpus at {vega}: only the repository fixtures are included", file=sys.stderr)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT, "w", encoding="utf-8") as f:
    json.dump(examples, f, ensure_ascii=False, separators=(",", ":"))
print(f"{len(examples)} examples, {os.path.getsize(OUT) // 1024} kB -> {OUT}")
