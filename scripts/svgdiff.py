#!/usr/bin/env python3
"""Compare two SVG files structurally: svgdiff.py ref.svg got.svg [tolerance]."""
import re
import sys
import xml.etree.ElementTree as ET

NUM = re.compile(r"-?\d+(?:\.\d+)?")


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
        va, vb = a.attrib.get(k), b.attrib.get(k)
        if va is None or vb is None or not same(va, vb, tol):
            out.append(f"{path}@{k}: ref={va!r} got={vb!r}")
    ta, tb = (a.text or "").strip(), (b.text or "").strip()
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
