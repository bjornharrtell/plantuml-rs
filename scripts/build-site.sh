#!/bin/sh
# Builds the wasm package into site/pkg; serve with `python3 -m http.server -d site`.
set -e
cd "$(dirname "$0")/.."
cargo build -p plantuml-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir site/pkg \
  target/wasm32-unknown-unknown/release/plantuml_wasm.wasm
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o site/pkg/plantuml_wasm_bg.wasm site/pkg/plantuml_wasm_bg.wasm
fi
git rev-parse --short HEAD > site/pkg/version.txt 2>/dev/null || echo dev > site/pkg/version.txt
ls -l site/pkg
