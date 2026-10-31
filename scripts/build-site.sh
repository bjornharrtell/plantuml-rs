#!/bin/sh
# Builds the wasm package into site/pkg; serve with `python3 -m http.server -d site`.
set -e
cd "$(dirname "$0")/.."
cargo build -p plantuml-wasm --target wasm32-unknown-unknown --profile wasm
wasm-bindgen --target web --no-typescript --out-dir site/pkg \
  target/wasm32-unknown-unknown/wasm/plantuml_wasm.wasm
# wasm-opt -Oz shrinks the raw file (4.4 MB to 3.8 MB) but the gzipped size grows (1.48 MB to 1.56 MB), so it is not used.
python3 scripts/build-examples.py site/examples.json
git rev-parse --short HEAD > site/pkg/version.txt 2>/dev/null || echo dev > site/pkg/version.txt
ls -l site/pkg
