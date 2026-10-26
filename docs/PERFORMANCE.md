# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Both render
the same corpus in one process, so the numbers measure rendering, not process or JVM startup.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of three warm runs. Host: 8 vCPU,
15 GB RAM, Linux. Toolchain: rustc 1.99.1, OpenJDK 21.0.13 (Temurin LTS), PlantUML 1.2026.9.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 1.81 s | 3.82 ms | 261 |
| Java PlantUML, one JVM | 1.31 s | 2.77 ms | 361 |

Java is about 1.4x faster per diagram on this corpus. Two caveats: Java writes its SVGs to `/dev/shm`
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

## Where the time goes

Profiled with `perf` (frame pointers plus debug info; `perf record -F 1999 -g --call-graph fp` over
eight passes of the 149 sequence diagrams). Ranked self time before the first optimisation:

| cost | share | cause |
|---|---|---|
| `DefaultHasher` + SipHash rounds | ~20% | `style::StyleIndex::find_matching` rebuilt its trie and hashed every atom of every style on every lookup |
| `malloc`/`free` and friends | ~16% | allocation churn from the same trie plus style merging |
| regex-automata NFA construction | ~12% | lazy DFA caches built during matching inside fancy-regex |

Landed:

1. **Style index trie built once.** `StyleIndex` now caches its signature trie in a `OnceCell`; the
   lookup no longer rebuilds it. Sequence corpus 1040 -> 560 ms, whole corpus 2550 -> 1810 ms
   (5.4 -> 3.8 ms per diagram), fidelity untouched.
2. **`format_decimal` in one pass**, without the intermediate `String` and `insert` calls that ran for
   every coordinate of the SVG (about 3% on the sequence corpus).

Next target: the regex machinery. Simple patterns (no lookaround, no backreference) could go through
`regex::Regex`, which keeps a thread-local cache pool, instead of `fancy-regex`, which builds its NFA
state caches per match; that is the remaining ~12% plus part of the allocation churn.

Text measurement is not a factor: the whole sequence run performs 170,466 measurements in 11 ms.

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
