#!/usr/bin/env python3
"""Render a corpus of `.puml` files and report timings.

Compares the native Rust CLI (one process per diagram), the Rust library rendering in one
process, and Java PlantUML (one JVM, and one process per diagram).

Usage:
    scripts/bench.py --corpus DIR --rust-cli PATH [--rust-example PATH] \
        [--java PATH] [--plantuml-jar PATH] [--runs N] [--java-per-file N]
"""
import argparse
import glob
import os
import shutil
import subprocess
import sys
import tempfile
import time


def corpus_files(root):
    return sorted(glob.glob(os.path.join(root, "**", "*.puml"), recursive=True))


def run_timed(label, fn, runs):
    best = None
    for _ in range(runs):
        start = time.perf_counter()
        ok = fn()
        elapsed = time.perf_counter() - start
        print(f"{label}: {elapsed * 1000:.0f} ms, {ok} ok", file=sys.stderr)
        best = elapsed if best is None else min(best, elapsed)
    return best


def bench_rust_cli(cli, files, runs):
    def once():
        ok = 0
        for path in files:
            r = subprocess.run([cli, os.path.basename(path)], cwd=os.path.dirname(path),
                               capture_output=True)
            ok += r.returncode == 0
        return ok

    return run_timed("rust cli (one process per diagram)", once, runs)


def bench_rust_example(example, corpus, runs):
    def once():
        out = subprocess.run([example, corpus], capture_output=True, text=True).stdout
        return out.strip().split("ok=")[1].split()[0] if "ok=" in out else "?"

    return run_timed("rust in-process", once, runs)


def bench_java(java, jar, files, runs):
    outdir = tempfile.mkdtemp(prefix="puml-bench-")

    def once():
        subprocess.run([java, "-jar", jar, "-tsvg", "-o", outdir, *files], capture_output=True)
        return sum(1 for _ in glob.glob(os.path.join(outdir, "**", "*.svg"), recursive=True))

    try:
        return run_timed("java (one jvm)", once, runs)
    finally:
        shutil.rmtree(outdir, ignore_errors=True)


def bench_java_per_file(java, jar, files, runs, sample):
    outdir = tempfile.mkdtemp(prefix="puml-bench-")
    picked = files[:sample]

    def once():
        for path in picked:
            subprocess.run([java, "-jar", jar, "-tsvg", "-o", outdir, path], capture_output=True)
        return len(picked)

    try:
        elapsed = run_timed("java (one process per diagram, sample)", once, runs)
        return elapsed / len(picked) * len(files)
    finally:
        shutil.rmtree(outdir, ignore_errors=True)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--corpus", default=os.environ.get("VEGA_DIR", ""))
    p.add_argument("--rust-cli", required=True)
    p.add_argument("--rust-example")
    p.add_argument("--java")
    p.add_argument("--plantuml-jar")
    p.add_argument("--runs", type=int, default=3)
    p.add_argument("--java-per-file", type=int, default=10)
    a = p.parse_args()
    files = corpus_files(a.corpus)
    if not files:
        sys.exit(f"no .puml files below {a.corpus}")
    print(f"corpus: {len(files)} diagrams", file=sys.stderr)

    rows = []
    rust_cli = bench_rust_cli(a.rust_cli, files, a.runs)
    rows.append(("rust cli, one process per diagram", rust_cli))
    if a.rust_example:
        rows.append(("rust library, one process", bench_rust_example(a.rust_example, a.corpus, a.runs)))
    if a.java and a.plantuml_jar:
        rows.append(("java plantuml, one jvm", bench_java(a.java, a.plantuml_jar, files, a.runs)))
        rows.append(("java plantuml, one process per diagram (extrapolated)",
                     bench_java_per_file(a.java, a.plantuml_jar, files, a.runs, a.java_per_file)))

    print()
    print("| run mode | total (s) | ms per diagram | diagrams/s |")
    print("|---|---|---|---|")
    for label, elapsed in rows:
        print(f"| {label} | {elapsed:.2f} | {elapsed / len(files) * 1000:.2f} | {len(files) / elapsed:.0f} |")


if __name__ == "__main__":
    main()
