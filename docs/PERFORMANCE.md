# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Both render
the same corpus in one process, so the numbers measure rendering, not process or JVM startup.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of three warm runs. Host: 8 vCPU,
15 GB RAM, Linux. Toolchain: rustc 1.99.1, OpenJDK 21.0.13 (Temurin LTS), PlantUML 1.2026.9.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 2.69 s | 5.70 ms | 176 |
| Java PlantUML, one JVM | 1.31 s | 2.77 ms | 361 |

Java is about 2x faster per diagram on this corpus. Two caveats: Java writes its SVGs to `/dev/shm`
here while the Rust harness keeps them in memory, and class diagrams are not like-for-like — this port
lays them out with `graphviz-pure`, while the Java run used Graphviz through its own DOT round trip.

## Where the time goes

Per type, measured in one process (Rust), and the share of the 2.55 s total the same way the
corpus run breaks down:

| type | diagrams | total | share | per diagram |
|---|---|---|---|---|
| sequence | 149 | 1.08 s | 42% | 7.2 ms |
| state | 48 | 0.49 s | 19% | 10.1 ms |
| activity | 36 | 0.45 s | 18% | 12.6 ms |
| class | 53 | 0.45 s | 18% | 8.4 ms |
| chart | 35 | 0.09 s | 4% | 2.6 ms |

The Java side cannot be split the same way: a per-type run starts a JVM and pays JIT compilation
on a few dozen diagrams, which is a large and uneven share, so only the whole-corpus figure (2.77 ms
per diagram in one JVM) is meaningful.

What makes the port slower overall is therefore not one hotspot in the layout: sequence diagrams are
the largest block (a third of the corpus) and the slowest per diagram, while the Graphviz-backed types
are within a factor of 1.2-1.5 of Java. The suspects, in order, are the per-string text measurement
against the embedded tables (no memoisation, Java caches AWT metrics), the regex work each source line
goes through (fancy-regex backtracks where Java's patterns do not), and the allocation-heavy tile
trees (every segment and label is a boxed `Rc<dyn TextBlock>`), all costs a JIT would flatten.

## One-time initialization

The Rust library spends about 76 ms on its first render in a process (font tables, style parsing and
regex compilation) and about 1 ms per tiny diagram afterwards. That cost is paid once per process or
per wasm instance, so it matters for one-shot use and not for a server that renders continuously.

## Reproducing

```sh
cargo build --release -p plantuml-cli
cargo build --release -p plantuml-core --example render_corpus
VEGA_DIR=/path/to/plantuml/src/test/resources/vega \
  scripts/bench.py --corpus "$VEGA_DIR" \
    --rust-example target/release/examples/render_corpus \
    --java "$(command -v java)" --plantuml-jar plantuml.jar
```

The `rust_example` row of `scripts/bench.py` is the library figure above; its CLI rows measure one
process per diagram and are there only for completeness.
