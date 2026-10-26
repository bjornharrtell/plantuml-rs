#!/usr/bin/env node
// Renders a corpus with the wasm build in one process and reports timings.
//
// Build the package first:
//   cargo build -p plantuml-wasm --target wasm32-unknown-unknown --release
//   wasm-bindgen --target nodejs --out-dir /tmp/wasmpkg \
//     target/wasm32-unknown-unknown/release/plantuml_wasm.wasm
// then: node scripts/bench-wasm.mjs /tmp/wasmpkg <corpus-dir> [runs]
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const [pkgDir, corpus, runsArg] = process.argv.slice(2);
if (!pkgDir || !corpus) {
  console.error('usage: bench-wasm.mjs <wasm-bindgen out dir> <corpus dir> [runs]');
  process.exit(2);
}
const runs = Number(runsArg || 3);
const { render } = require(path.join(pkgDir, 'plantuml_wasm.js'));

const files = [];
(function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(p);
    else if (entry.name.endsWith('.puml')) files.push(p);
  }
})(corpus);
files.sort();
const sources = files.map((f) => fs.readFileSync(f, 'utf8'));

const initStart = process.hrtime.bigint();
try { render('@startuml\nA -> B\n@enduml'); } catch (e) {}
console.error(`first render (module init included): ${(Number(process.hrtime.bigint() - initStart) / 1e6).toFixed(0)} ms`);

let best = Infinity;
for (let run = 0; run < runs; run++) {
  let ok = 0;
  const start = process.hrtime.bigint();
  for (const src of sources) { try { render(src); ok++; } catch (e) {} }
  const ms = Number(process.hrtime.bigint() - start) / 1e6;
  best = Math.min(best, ms);
  console.error(`run ${run}: ${ms.toFixed(0)} ms, ${ok} ok`);
}
console.log(`wasm: ${files.length} diagrams in ${(best / 1000).toFixed(2)} s, ${(best / files.length).toFixed(2)} ms per diagram, ${(files.length / (best / 1000)).toFixed(0)} diagrams/s`);
