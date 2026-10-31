#!/bin/sh
# Builds the npm package into npm/ (not committed except its manifest and README).
#   scripts/build-npm.sh          build the two wasm targets and stage the package
#   scripts/build-npm.sh -p       also run `npm pack` to validate the tarball
set -e
cd "$(dirname "$0")/.."
target_dir="${CARGO_TARGET_DIR:-target}"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

cargo build -p plantuml-wasm --target wasm32-unknown-unknown --profile wasm
wasm="$target_dir/wasm32-unknown-unknown/wasm/plantuml_wasm.wasm"

rm -rf npm/dist
mkdir -p npm/dist/node npm/dist/web
# Node/CommonJS build, used by scripts and agents.
wasm-bindgen --target nodejs --out-dir npm/dist/node "$wasm"
# Browser/bundler ESM build: call init() (or initSync with the bytes) once. The marker file keeps
# Node from reading the ESM glue as CommonJS when the web build is imported directly.
wasm-bindgen --target web --out-dir npm/dist/web "$wasm"
printf '{\n  "type": "module"\n}\n' > npm/dist/web/package.json

cp LICENSE npm/LICENSE
sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" npm/package.json
rm -f npm/package.json.bak

ls -l npm/dist/node/plantuml_wasm_bg.wasm npm/dist/web/plantuml_wasm_bg.wasm

if [ "$1" = "-p" ]; then
  (cd npm && npm pack --pack-destination .)
fi
