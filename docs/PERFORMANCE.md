# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Both render the
same corpus in one process, so the numbers measure rendering, not process or JVM startup.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of three warm runs. Host: 8 vCPU,
15 GB RAM, Linux. Toolchain: rustc 1.99.1, OpenJDK 21.0.13 (Temurin LTS), PlantUML 1.2026.9.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 1.45 s | 3.07 ms | 326 |
| Java PlantUML, one JVM | 1.32 s | 2.80 ms | 358 |

Java is about 10% faster per diagram on this corpus. Two caveats: Java writes its SVGs to `/dev/shm`
while the Rust harness keeps them in memory, and class diagrams are not like-for-like — this port lays
them out with `graphviz-pure`, while the Java run used Graphviz through its own DOT round trip. The Java
side also cannot be split per diagram type the same way, because a per-type run pays JVM start and JIT
compilation on a few dozen diagrams.

## Per diagram type

Rust, one process per type, so each row also pays the one-time initialization described below.

| type | diagrams | total | per diagram |
|---|---|---|---|
| sequence | 149 | 483 ms | 3.2 ms |
| class | 53 | 356 ms | 6.7 ms |
| state | 48 | 390 ms | 8.1 ms |
| activity | 36 | 347 ms | 9.6 ms |
| chart | 35 | 66 ms | 1.9 ms |

The Graphviz-backed types are the slowest per diagram; sequence diagrams are the largest block of the
corpus and therefore dominate the total.

## Profile

Profiled with `perf` (frame pointers plus debug info, `perf record -F 1999 -g --call-graph fp` over
eight passes of the sequence corpus). The costs that matter today:

| cost | share | cause |
|---|---|---|
| allocator (`malloc`/`free` and friends) | ~18% | label strings, style merging, regex search caches |
| regex-automata NFA and cache construction | ~8% | `fancy-regex` builds a search cache per call; it cannot be reused from outside that crate |
| string scanning and comparison | ~4% | command matching over the source lines |

Text measurement is not a factor: the sequence corpus performs 170,466 measurements in 11 ms.

Two obvious optimisations are measured and do not help, so the code stays simple: routing patterns
without lookaround through `regex::Regex` instead of `fancy-regex`, and a first-token prefilter on the
command list (Java's `mandatoryFirstTokensFast`). The remaining distance to Java is JIT against AOT —
`Rc<dyn TextBlock>` dispatch and `Vec` growth are inlined there and cannot be here — so closing it needs
an upstream change to `fancy-regex` or a broad allocation pass with a yield below the gap.

## One-time initialization

The library spends about 75 ms on its first render in a process (font tables, style parsing and regex
compilation) and about 1 ms per tiny diagram afterwards. That cost is paid once per process or per wasm
instance, so it matters for one-shot use and not for a server that renders continuously.

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
