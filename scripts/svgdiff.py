#!/usr/bin/env python3
"""Compare two SVG files structurally: svgdiff.py ref.svg got.svg [tolerance]."""
import html
import re
import sys
import xml.etree.ElementTree as ET

NUM = re.compile(r"-?\d+(?:\.\d+)?")

# Attributes that carry no visual meaning: source line attribution and similar metadata.
IGNORED_ATTRIBUTES = {"data-source-line"}


def _text(el):
    """Text content with XML entities decoded, so `&gt;` and `>` compare equal."""
    return html.unescape((el.text or "").strip())


FILTER_ID = re.compile(r'^(url\(#)?f[0-9a-z]{2,14}(\))?$')


def norm_value(v):
    # filter ids are random in Java
    m = FILTER_ID.match(v) if v else None
    if m:
        return 'FILTERID'
    return v


def same(a, b, tol):
    a, b = norm_value(a), norm_value(b)
    if a == b:
        return True
    na, nb = NUM.findall(a), NUM.findall(b)
    if len(na) == len(nb) and NUM.sub("#", a) == NUM.sub("#", b):
        return all(abs(float(x) - float(y)) <= tol for x, y in zip(na, nb))
    return False


def strip(tag):
    return tag.split("}")[-1]


def walk(a, b, path, tol, out):
    if strip(a.tag) != strip(b.tag):
        out.append(f"{path}: tag {strip(a.tag)} != {strip(b.tag)}")
        return
    path = f"{path}/{strip(a.tag)}"
    for k in sorted(set(a.attrib) | set(b.attrib)):
        if k in IGNORED_ATTRIBUTES:
            continue
        va, vb = a.attrib.get(k), b.attrib.get(k)
        if va is None or vb is None or not same(va, vb, tol):
            out.append(f"{path}@{k}: ref={va!r} got={vb!r}")
    ta, tb = _text(a), _text(b)
    if ta != tb:
        out.append(f"{path} text: ref={ta!r} got={tb!r}")
    ca, cb = list(a), list(b)
    for i, (x, y) in enumerate(zip(ca, cb)):
        walk(x, y, f"{path}[{i}]", tol, out)
    if len(ca) != len(cb):
        out.append(f"{path}: {len(ca)} children in ref, {len(cb)} in got")


def differences(ref_root, got_root, tol=0.002):
    """Structural differences between two SVG element trees; empty means identical."""
    out = []
    walk(ref_root, got_root, "", tol, out)
    return out


def _length(root, name):
    m = NUM.search(root.attrib.get(name, "") or "")
    return float(m.group(0)) if m else None


def _canvas(root):
    w, h = _length(root, "width"), _length(root, "height")
    if w and h:
        return w, h
    box = NUM.findall(root.attrib.get("viewBox", ""))
    if len(box) == 4:
        return float(box[2]), float(box[3])
    return None, None


def _center(el):
    """Centre of a leaf shape, in the coordinates of the document."""
    tag, a = strip(el.tag), el.attrib

    def f(k):
        try:
            return float(a[k])
        except (KeyError, ValueError):
            return None

    if tag == "rect":
        return (f("x") or 0.0) + (f("width") or 0.0) / 2, (f("y") or 0.0) + (
            f("height") or 0.0
        ) / 2
    if tag in ("ellipse", "circle"):
        return f("cx") or 0.0, f("cy") or 0.0
    if tag == "line":
        return ((f("x1") or 0.0) + (f("x2") or 0.0)) / 2, (
            (f("y1") or 0.0) + (f("y2") or 0.0)
        ) / 2
    if tag == "text":
        return f("x") or 0.0, f("y") or 0.0
    if tag in ("path", "polygon", "polyline"):
        nums = [float(v) for v in NUM.findall(a.get("d") or a.get("points") or "")]
        if len(nums) >= 2:
            xs, ys = nums[0::2], nums[1::2]
            return (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    return None


def _elements(root):
    """Leaf shapes keyed by tag, class and text, with their centre."""
    out = {}
    for el in root.iter():
        c = _center(el)
        if c is None:
            continue
        key = (strip(el.tag), el.attrib.get("class", ""), _text(el))
        out.setdefault(key, []).append(c)
    for positions in out.values():
        positions.sort()
    return out


def layout(ref_root, got_root, tol=0.03):
    """Compare where elements sit relative to the canvas, ignoring font metrics.

    Returns (matched_fraction, mean_error): a reference element counts as matched when an
    element of the same tag, class and text sits within `tol` times the canvas diagonal.
    Sizes, text widths and glyph shapes are deliberately not compared.
    """
    rw, rh = _canvas(ref_root)
    gw, gh = _canvas(got_root)
    if not rw or not rh or not gw or not gh:
        return 0.0, None
    ref, got = _elements(ref_root), _elements(got_root)
    total = sum(len(v) for v in ref.values())
    if total == 0:
        return 0.0, None
    matched = 0
    errors = []
    for key, positions in ref.items():
        others = got.get(key, [])
        for i, (rx, ry) in enumerate(positions):
            if i >= len(others):
                break
            gx, gy = others[i]
            error = (
                ((rx / rw) - (gx / gw)) ** 2 + ((ry / rh) - (gy / gh)) ** 2
            ) ** 0.5
            errors.append(error)
            if error <= tol:
                matched += 1
    mean = sum(errors) / len(errors) if errors else None
    return matched / total, mean


def main():
    ref = ET.parse(sys.argv[1]).getroot()
    got = ET.parse(sys.argv[2]).getroot()
    tol = float(sys.argv[3]) if len(sys.argv) > 3 else 0.002
    out = differences(ref, got, tol)
    for line in out[:40]:
        print(line)
    print("IDENTICAL" if not out else f"{len(out)} differences")


if __name__ == "__main__":
    main()
