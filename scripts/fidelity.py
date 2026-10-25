import os, re, subprocess, json, sys, html, collections, xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import svgdiff

ROOT = os.environ.get("VEGA_DIR", "/home/bjorn/code/plantuml/src/test/resources/vega")
PUML = sys.argv[1]  # renderer command reading a diagram on stdin
TOL = float(os.environ.get("SVG_TOL", "0.5"))  # coordinate tolerance in pixels
LAYOUT_TOL = float(os.environ.get("LAYOUT_TOL", "0.03"))  # relative to the canvas
out = []
for dp, _, fs in os.walk(ROOT):
    for f in fs:
        if not f.endswith(".puml"):
            continue
        p = os.path.join(dp, f)
        svgp = p[:-5] + ".svg"
        if not os.path.exists(svgp):
            continue
        src = open(p, encoding="utf-8").read()
        m = re.search(r"^@start", src, re.M)
        if not m:
            continue
        body = src[m.start():]
        ref = open(svgp, encoding="utf-8").read()
        t = re.search(r'data-diagram-type="([^"]+)"', ref)
        typ = t.group(1) if t else "?"
        try:
            r = subprocess.run([PUML, "-"], input=body.encode(), capture_output=True, timeout=20)
            ok = r.returncode == 0 and b"<svg" in r.stdout
            got = r.stdout.decode("utf-8", "replace")
        except Exception:
            ok, got = False, ""

        def texts(s):
            return collections.Counter(
                html.unescape(re.sub(r"<[^>]+>", "", x)).strip()
                for x in re.findall(r"<text[^>]*>(.*?)</text>", s, re.S)
                if re.sub(r"<[^>]+>", "", x).strip()
            )

        def dims(s):
            w = re.search(r'<svg[^>]*?\swidth="([\d.]+)', s)
            h = re.search(r'<svg[^>]*?\sheight="([\d.]+)', s)
            return (float(w.group(1)), float(h.group(1))) if w and h else None

        def struct_diffs(ref_text, got_text):
            """Number of structural differences from the reference SVG, None if unparsable."""
            try:
                return len(svgdiff.differences(
                    ET.fromstring(ref_text), ET.fromstring(got_text), TOL))
            except Exception:
                return None

        def layout_match(ref_text, got_text):
            """Share of reference elements whose relative position matches, font-agnostic."""
            try:
                share, mean = svgdiff.layout(
                    ET.fromstring(ref_text), ET.fromstring(got_text), LAYOUT_TOL)
                return share, mean
            except Exception:
                return None, None

        tr, tg = texts(ref), texts(got)
        inter = sum((tr & tg).values())
        uni = sum((tr | tg).values()) or 1
        dr, dg = dims(ref), dims(got) if ok else None
        wr = hr = None
        if dr and dg and dr[0] and dr[1]:
            wr, hr = dg[0] / dr[0], dg[1] / dr[1]
        lay = layout_match(ref, got) if ok else (None, None)
        out.append(dict(file=os.path.relpath(p, ROOT), type=typ, ok=ok,
                        text_jaccard=inter / uni, wr=wr, hr=hr,
                        diffs=struct_diffs(ref, got) if ok else None,
                        layout=lay[0], layout_error=lay[1]))

json.dump(out, open(os.environ.get("FIDELITY_JSON", "fidelity.json"), "w"))
by = collections.defaultdict(list)
for o in out:
    by[o["type"]].append(o)
print("type n ok% text_jaccard_mean size_within20% layout_mean identical%")
for k, v in sorted(by.items(), key=lambda kv: -len(kv[1])):
    n = len(v)
    ok = sum(o["ok"] for o in v) / n
    tj = sum(o["text_jaccard"] for o in v) / n
    sz = sum(1 for o in v if o["wr"] and abs(o["wr"] - 1) < .2 and abs(o["hr"] - 1) < .2) / n
    ident = sum(1 for o in v if o["diffs"] == 0) / n
    lay = sum(o["layout"] or 0 for o in v) / n
    print(f"{k:14s} {n:4d} {ok*100:5.0f} {tj:5.2f} {sz*100:5.0f} {lay:5.2f} {ident*100:5.0f}")
n = len(out)
print("TOTAL", n, sum(o["ok"] for o in out) / n, sum(o["text_jaccard"] for o in out) / n,
      f"layout={sum(o['layout'] or 0 for o in out) / n:.2f}",
      f"identical={sum(1 for o in out if o['diffs'] == 0)}")
