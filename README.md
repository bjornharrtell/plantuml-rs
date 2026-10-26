# plantuml-rs

Rust/wasm port of [PlantUML](https://github.com/plantuml/plantuml): renders diagrams to SVG in the
browser with no backend. See [docs/PLAN.md](docs/PLAN.md) for the porting decisions, the fidelity
target and the current per-type status.

```sh
cargo run --release -p plantuml-cli -- diagram.puml > diagram.svg
```

## Status

391 of 405 scored Vega fixtures meet the fidelity target (text overlap >= 0.95, relative element
placement >= 0.90), 327 are structurally identical, and every one of the 431 reference fixtures
renders. Supported today: sequence, class (with object and description diagrams), state, activity,
packet, chart, gantt, timing, mindmap, WBS, salt, JSON/YAML/HCL, Chen EER, EBNF and the crash
diagram. C4-PlantUML renders through the bundled stdlib. Accepted differences and open items are
listed in the plan.

## Performance

Measured on the 472 `.puml` files of the PlantUML Vega corpus, single run per diagram, best of three
warm runs. Host: 8 vCPU, 15 GB RAM, Linux. Toolchain: rustc 1.99.1, OpenJDK 21.0.13 (Temurin LTS),
PlantUML 1.2026.9, Node 22.14.0.

| run mode | total | ms per diagram | diagrams/s |
|---|---|---|---|
| Rust library, one process (`examples/render_corpus`) | 2.69 s | 5.70 | 176 |
| Java PlantUML, one JVM (`-tsvg`) | 1.31 s | 2.77 | 361 |
| wasm build, one process (Node 22) | 4.00 s | 8.47 | 118 |
| Rust CLI, one process per diagram | 59.1 s | 125.2 | 8 |
| Java PlantUML, one process per diagram | 197.6 s | 418.6 | 2 |

What the numbers mean:

- **Rendering throughput** is the first two rows: one process, all diagrams. Java is about 2x
  faster per diagram than the port on this corpus; the wasm build is about 1.5x slower than the
  native library. Java writes its SVGs to `/dev/shm` in this run while the Rust harness keeps them
  in memory, so the gap is, if anything, slightly smaller than the table shows.
- **One-shot CLI use** is dominated by startup, not rendering: the Rust binary spends about 76 ms
  in one-time initialization (table decoding, regex compilation, style parsing) and about 5 ms on a
  typical diagram, while a JVM start costs about 400 ms. For batch use the JVM wins; for a single
  diagram from a script the Rust CLI is about 3x faster end to end.
- 456 of the 472 fixtures produce an SVG in the Java and wasm runs and 458 in the native ones; the
  rest fail on the same inputs (error cases, includes that need a filesystem).

Artifact sizes (the wasm numbers are before `wasm-opt`):

| build | size |
|---|---|
| `plantuml_wasm.wasm`, raw | 5.91 MB |
| `plantuml_wasm.wasm`, gzip -9 | 1.88 MB |
| TeaVM build of Java PlantUML, `plantuml.js` | 3.11 MB |
| TeaVM build of Java PlantUML, `plantuml.wasm` (Wasm GC) | 4.30 MB |

Limits of the comparison: the TeaVM engines are not benchmarked here. They measure text through a
browser DOM (`getBBox`, a canvas context), so driving them headlessly needs shims that change what is
measured; the upstream `tevm-wasm.sh` builds `bench.html` for that comparison in a browser. Class
diagrams are also not like-for-like across engines: this port lays them out with `graphviz-pure`,
while the TeaVM build falls back to Smetana unless Viz.js is loaded.

### Reproducing

```sh
# native: Rust CLI, Rust library in one process, Java single JVM, Java per process
cargo build --release -p plantuml-cli
cargo build --release -p plantuml-core --example render_corpus
VEGA_DIR=/path/to/plantuml/src/test/resources/vega \
  scripts/bench.py --corpus "$VEGA_DIR" \
    --rust-cli target/release/plantuml-cli \
    --rust-example target/release/examples/render_corpus \
    --java "$(command -v java)" --plantuml-jar plantuml.jar

# wasm: build the package, then render the corpus in one Node process
cargo build -p plantuml-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs --out-dir /tmp/wasmpkg \
  target/wasm32-unknown-unknown/release/plantuml_wasm.wasm
node scripts/bench-wasm.mjs /tmp/wasmpkg "$VEGA_DIR"
```

## License

MIT. Ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0.
