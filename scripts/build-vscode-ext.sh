#!/bin/sh
# Builds the wasm package into editors/vscode/wasm and, given -p, packages the VS Code extension.
set -e
cd "$(dirname "$0")/.."
target_dir="${CARGO_TARGET_DIR:-target}"
cargo build -p plantuml-wasm --target wasm32-unknown-unknown --profile wasm
wasm-bindgen --target nodejs --out-dir editors/vscode/wasm \
  "$target_dir/wasm32-unknown-unknown/wasm/plantuml_wasm.wasm"
ls -l editors/vscode/wasm/plantuml_wasm_bg.wasm
if [ "$1" = "-p" ]; then
  cd editors/vscode
  npm install --no-audit --no-fund
  npx --yes @vscode/vsce package --out plantuml-rs.vsix
fi
