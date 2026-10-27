import os, re, subprocess, json, sys, html, collections, fnmatch, xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import svgdiff

HERE = os.path.dirname(os.path.abspath(__file__))
_args = sys.argv[1:]
_extra_roots = [a[len("--root="):] for a in _args if a.startswith("--root=")]
_java = [a[len("--java="):] for a in _args if a.startswith("--java=")]
_jar = [a[len("--jar="):] for a in _args if a.startswith("--jar=")]
_jobs = [a[len("--jobs="):] for a in _args if a.startswith("--jobs=")]
_plain = [a for a in _args if not a.startswith(("--root=", "--java=", "--jar=", "--jobs="))]
PUML = os.path.abspath(_plain[0])  # renderer command reading a diagram on stdin

# With `--java` and `--jar` the references come from a live PlantUML render of the same source instead of
# the stored `.svg` files: that is the oracle for what PlantUML ships today, where the stored corpus was
# produced by its test mode (SVG_DETERMINISTIC, Smetana layout). See docs/PLAN.md.
JAVA = _java[0] if _java else None
JAR = _jar[0] if _jar else None
if bool(JAVA) != bool(JAR):
    sys.exit("--java and --jar must be given together")
LIVE = bool(JAVA and JAR)
JOBS = int(_jobs[0]) if _jobs else min(8, os.cpu_count() or 4)
TOL = float(os.environ.get("SVG_TOL", "0.5"))  # coordinate tolerance in pixels
LAYOUT_TOL = float(os.environ.get("LAYOUT_TOL", "0.03"))  # relative to the canvas

# Roots scored by this harness: the Vega corpus, any `--root=`, and the repository fixtures for the diagram
# types the Vega corpus does not cover. Fixture paths are reported with a `fixtures/` prefix.
ROOTS = []
VEGA = os.environ.get("VEGA_DIR", "/home/bjorn/code/plantuml/src/test/resources/vega")
if os.path.isdir(VEGA):
    ROOTS.append((VEGA, ""))
for _root in _extra_roots:
    ROOTS.append((os.path.abspath(_root), ""))
_FIXTURES = os.path.join(os.path.dirname(HERE), "fixtures")
if os.path.isdir(_FIXTURES):
    ROOTS.append((_FIXTURES, "fixtures/"))


def load_accepted():
    """Fixtures whose differences are accepted (see docs/PLAN.md), one `glob # reason` per line."""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "accepted-fidelity.txt")
    rules = []
    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            body = line.split("#", 1)[0].strip()
            if body:
                rules.append(body)
    return rules


def is_accepted(rules, relpath):
    return any(fnmatch.fnmatch(relpath, rule) for rule in rules)


ACCEPTED = load_accepted()
SMETANA = re.compile(r"!pragma\s+layout\s+smetana")
out = []


def corpus(root):
    found = []
    for dp, _, fs in os.walk(root):
        for f in fs:
            if f.endswith(".puml") and os.path.exists(os.path.join(dp, f)[:-5] + ".svg"):
                found.append(os.path.join(dp, f))
    return sorted(found)


def entries():
    """Every scored fixture with its source and the body the renderers get."""
    found = []
    for root, prefix in ROOTS:
        for p in corpus(root):
            src = open(p, encoding="utf-8").read()
            m = re.search(r"^@start", src, re.M)
            if not m:
                continue
            found.append(dict(root=root, prefix=prefix, path=p, src=src,
                              body=src[m.start():],
                              rel=prefix + os.path.relpath(p, root)))
    return found


def java_render(entry):
    """SVG for this source from a live PlantUML, or None when it does not render."""
    try:
        r = subprocess.run([JAVA, "-jar", JAR, "-tsvg", "-pipe"], input=entry["body"].encode(),
                           capture_output=True, timeout=180,
                           cwd=os.path.dirname(entry["path"]) or None)
    except Exception as e:
        return entry["rel"], None, str(e)
    svg = r.stdout.decode("utf-8", "replace")
    if r.returncode != 0 or "<svg" not in svg:
        lines = r.stderr.decode("utf-8", "replace").strip().splitlines()
        return entry["rel"], None, lines[-1] if lines else "no svg"
    return entry["rel"], svg, None


ENTRIES = entries()
JAVA_REFS = {}
REF_FAILED = []
if LIVE:
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(max_workers=JOBS) as pool:
        for rel, svg, err in pool.map(java_render, ENTRIES):
            if svg is None:
                REF_FAILED.append((rel, err))
            else:
                JAVA_REFS[rel] = svg
    print(f"reference: live Java ({JAR}), {len(JAVA_REFS)} rendered, {len(REF_FAILED)} failed",
          file=sys.stderr)
    for rel, err in REF_FAILED:
        print(f"  reference failed: {rel}: {err}", file=sys.stderr)

for e in ENTRIES:
    p, prefix, root, src, body = e["path"], e["prefix"], e["root"], e["src"], e["body"]
    rel = e["rel"]
    if LIVE:
        ref = JAVA_REFS.get(rel)
        if ref is None:
            continue
    else:
        ref = open(p[:-5] + ".svg", encoding="utf-8").read()
    t = re.search(r'data-diagram-type="([^"]+)"', ref)
    typ = t.group(1) if t else "?"
    try:
        r = subprocess.run([PUML, "-"], input=body.encode(), capture_output=True, timeout=20,
                           cwd=os.path.dirname(p) or None)
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
    text_score = inter / uni
    layout_score = lay[0]
    passes = ok and text_score >= 0.95 and (layout_score or 0) >= 0.90
    smetana = bool(SMETANA.search(src)) and not passes
    out.append(dict(file=rel, type=typ, ok=ok,
                    text_jaccard=text_score, wr=wr, hr=hr,
                    diffs=struct_diffs(ref, got) if ok else None,
                    layout=layout_score, layout_error=lay[1],
                    accepted=(smetana or (is_accepted(ACCEPTED, rel) and not passes)),
                    smetana=smetana, reference="java" if LIVE else "stored"))

json.dump(out, open(os.environ.get("FIDELITY_JSON", "fidelity.json"), "w"))
by = collections.defaultdict(list)
for o in out:
    by[o["type"]].append(o)

def passes(o):
    return o["ok"] and (o["text_jaccard"] or 0) >= 0.95 and (o["layout"] or 0) >= 0.90

print("type n ok% text_jaccard_mean layout_mean identical% pass accepted")
for k, v in sorted(by.items(), key=lambda kv: -len(kv[1])):
    n = len(v)
    ok = sum(o["ok"] for o in v) / n
    tj = sum(o["text_jaccard"] for o in v) / n
    lay = sum(o["layout"] or 0 for o in v) / n
    ident = sum(1 for o in v if o["diffs"] == 0) / n
    scored = [o for o in v if not o["accepted"]]
    acc = len(v) - len(scored)
    p = sum(1 for o in scored if passes(o))
    share = f"{p}/{len(scored)}" if scored else "-"
    print(f"{k:14s} {n:4d} {ok*100:5.0f} {tj:5.2f} {lay:5.2f} {ident*100:5.0f} "
          f"{share:>7s} {acc:3d}")
scored = [o for o in out if not o["accepted"]]
smetana = sum(1 for o in out if o["smetana"])
declared = len(out) - len(scored) - smetana
# Declared fixtures that already meet the target are scored like any other, so the denominator moves with
# the metrics. Report them separately to keep the totals comparable.
declared_passing = sum(1 for o in out if is_accepted(ACCEPTED, o["file"])) - declared
print(f"TOTAL {len(out)} fixtures, {len(out) - len(scored)} accepted "
      f"({smetana} smetana layout, {declared} declared in scripts/accepted-fidelity.txt), "
      f"{declared_passing} declared fixtures already pass, "
      f"{sum(1 for o in scored if passes(o))}/{len(scored)} in-scope pass, "
      f"layout={sum(o['layout'] or 0 for o in out) / len(out):.2f}, "
      f"identical={sum(1 for o in out if o['diffs'] == 0)}")
