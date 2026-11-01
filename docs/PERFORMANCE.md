# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Each run renders the whole
corpus in one process, so neither side pays process or JVM startup per diagram.

Measured on the 472 `.puml` files of the PlantUML Vega corpus (458 render on the Rust side, 456 on the Java side; the
rest need includes or are intentionally invalid), best of three warm runs, on mains power. Host: Linux, 16 vCPU,
29 GB RAM. Toolchain: rustc 1.99.1, OpenJDK 25.0.4.1, graphviz-pure 1.0.0.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 0.80 s | 1.69 ms | 593 |
| Java PlantUML 1.2026.10beta1, one JVM | 1.21 s | 2.56 ms | 391 |

Rust is about 34% ahead. The Java figure includes one JVM start (about 130 ms) and its JIT warm-up. Two further
caveats: Java writes its SVGs to `/dev/shm` while the Rust harness keeps them in memory, and class diagrams are not
like-for-like (this port lays them out with `graphviz-pure`, while the Java run used Graphviz through its own DOT round
trip). The Java side cannot be split per diagram type the same way, because a per-type run pays JVM start and JIT
compilation on a few dozen diagrams. The ranking depends on the Java version: the older 1.2026.7 jar is about 20%
slower than the pinned one.

A laptop on battery can throttle and make the Rust figure 40% worse; measure on mains power.

## What the build does for speed

- The native release profile is `opt-level = 3`. The `wasm` profile (`cargo build --profile wasm`) keeps
  `opt-level = "z"` for size, which would cost about 30% of native speed.
- Case folding is skipped for Unicode classes that are already case-closed (`\w`, `\p{L}`, `\p{N}` and bracket classes
  made of them): folding them was a quarter of regex compile time.

## What is left

The remaining distance to a JIT-warmed JVM is mostly allocation (label strings, style merging, regex search caches) and
regex NFA construction in `fancy-regex`, which cannot be reused from outside that crate. Two obvious optimisations were
measured and do not help: routing patterns without lookaround through `regex::Regex`, and a first-token prefilter on
the command list (Java's `mandatoryFirstTokensFast`) in steady state. A prefilter would cut start-up, since regexes are
compiled lazily on first match, but adds real complexity.

## One-time initialization

The first render in a process takes about 31 ms (font tables, style parsing and compiling the regexes of the diagram
type), a first class diagram about 43 ms, and about 1 ms per tiny diagram afterwards.
The cost is paid once per process or per wasm instance, so it matters for one-shot use and not for a server that renders
continuously.

## Reproducing

```sh
cargo build --release -p plantuml-cli
cargo build --release -p plantuml-core --example render_corpus
VEGA_DIR=/path/to/plantuml/src/test/resources/vega \
  scripts/bench.py --corpus "$VEGA_DIR" \
    --rust-cli target/release/plantuml-cli \
    --rust-example target/release/examples/render_corpus \
    --java "$(command -v java)" --plantuml-jar plantuml.jar
```

The `rust library` row is the figure above; the CLI rows measure one process per diagram and are there only for
completeness.
