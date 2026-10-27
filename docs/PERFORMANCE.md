# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Both render the
same corpus in one process, so the numbers measure rendering, not process or JVM startup.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of three warm runs. Host: Linux,
8 vCPU, 15 GB RAM. Toolchain: rustc 1.99.1, OpenJDK 25.0.4.1, PlantUML 1.2026.7, graphviz-pure 0.2.2.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 1.25 s | 2.65 ms | 377 |
| Java PlantUML, one JVM | 1.39 s | 2.94 ms | 340 |

Rust is about 10% faster per diagram on this corpus. Two caveats: Java writes its SVGs to `/dev/shm`
while the Rust harness keeps them in memory, and class diagrams are not like-for-like — this port lays
them out with `graphviz-pure`, while the Java run used Graphviz through its own DOT round trip. The Java
side also cannot be split per diagram type the same way, because a per-type run pays JVM start and JIT
compilation on a few dozen diagrams.

## Per diagram type

Rust, one process per type, so each row also pays the one-time initialization described below.

| type | diagrams | total | per diagram |
|---|---|---|---|
| activity | 36 | 281 ms | 7.8 ms |
| state | 48 | 307 ms | 6.4 ms |
| class | 53 | 282 ms | 5.3 ms |
| sequence | 149 | 412 ms | 2.8 ms |
| chart | 35 | 54 ms | 1.5 ms |

Activity and the Graphviz-backed types (state, class) are the slowest per diagram; sequence diagrams are
the largest block of the corpus and therefore dominate the total.

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
