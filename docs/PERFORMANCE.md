# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Each run renders the
whole corpus in one process, so neither side pays process or JVM startup per diagram.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of five warm runs. Host: Linux,
16 vCPU, 29 GB RAM. Toolchain: rustc 1.99.1, OpenJDK 25.0.4.1, graphviz-pure 0.2.3.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 0.85 s | 1.79 ms | 559 |
| Java PlantUML 1.2026.10beta1, one JVM | 1.16 s | 2.46 ms | 407 |
| Java PlantUML 1.2026.7, one JVM | 1.42 s | 3.01 ms | 332 |

The ranking depends on the Java version: 1.2026.10beta1 renders this corpus about 20% faster than
1.2026.7, mostly on sequence diagrams. Rust is about 27% ahead of the beta and about 40% ahead of the
stable jar. Native builds use `opt-level = 3`; the wasm build uses the size-optimised `wasm` profile
(`opt-level = "z"`), which was the native profile before and cost about 30% (1.22 s). Both figures include one process start each — about 130 ms for the JVM and a few milliseconds
for the Rust process — and the JVM also warms up its JIT during the run. Two further caveats: Java writes
its SVGs to `/dev/shm` while the Rust harness keeps them in memory, and class diagrams are not
like-for-like — this port lays them out with `graphviz-pure`, while the Java run used Graphviz through its
own DOT round trip. The Java side also cannot be split per diagram type the same way, because a per-type
run pays JVM start and JIT compilation on a few dozen diagrams.

## Per diagram type

Rust, one process per type, so each row also pays the one-time initialization described below.

| type | diagrams | total | per diagram |
|---|---|---|---|
| activity | 36 | 212 ms | 5.9 ms |
| state | 48 | 242 ms | 5.0 ms |
| class | 53 | 216 ms | 4.1 ms |
| sequence | 149 | 315 ms | 2.1 ms |
| chart | 35 | 35 ms | 1.0 ms |

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

The library spends about 40 ms on its first render in a process (font tables, style parsing and regex
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
