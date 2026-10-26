# Rendering performance

Library-level figures for this port and for the Java implementation it is ported from. Both render
the same corpus in one process, so the numbers measure rendering, not process or JVM startup.

Measured on the 472 `.puml` files of the PlantUML Vega corpus, best of three warm runs. Host: 8 vCPU,
15 GB RAM, Linux. Toolchain: rustc 1.99.1, OpenJDK 21.0.13 (Temurin LTS), PlantUML 1.2026.9.

| implementation | total | per diagram | diagrams/s |
|---|---|---|---|
| Rust library (`plantuml-core`) | 1.45 s | 3.07 ms | 326 |
| Java PlantUML, one JVM | 1.32 s | 2.80 ms | 358 |

Java is about 10% faster per diagram on this corpus: 3.07 ms against 2.80 ms. Earlier revisions of this
page reported 5.70 ms for the port; the style-index and pattern-sharing fixes below account for the
difference. Two caveats: Java writes its SVGs to `/dev/shm`
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
3. **Compiled patterns are shared.** Command patterns were cloned out of the compile cache for every
   diagram, and cloning a `fancy-regex` `Regex` loses its internal cache pool, so the first match of
   every command rebuilt the automaton caches (9.5% of the profile, plus 8% in `IRegex::engine`). The
   cache now hands out `Rc<Regex>`, so one engine and its pool serve the whole process: sequence
   575 -> 490 ms, corpus 1840 -> 1470 ms.

Tried and rejected, both measured on the sequence and whole corpus:

- **Routing patterns without lookaround through `regex::Regex`** instead of `fancy-regex`: neutral
  (sequences 575 -> 580 ms, corpus unchanged), because the cost was the per-diagram cloning described
  above, not the engine choice.
- **A first-token prefilter on the command list** (Java's `mandatoryFirstTokensFast`: derive the
  required leading word from the pattern tree and skip commands that cannot start with the line's
  prefix). Also neutral (corpus 1.43-1.54 s against a 1.45-1.51 s baseline, overlapping runs): once
  the compiled patterns are shared, the surviving searches are cheap, and action, note and link
  commands have no certain first token to filter on.

Both were reverted rather than kept for no gain.

What is left is ~18% in the allocator and ~8% in `fancy-regex`'s per-search cache construction, which
cannot be reused from outside that crate; closing it needs an upstream change there or a broad
allocation pass over labels and style merging, with an expected yield below the ~12% gap to Java. The
rest of the gap is JIT against AOT: `Rc<dyn TextBlock>` calls and `Vec` growth are inlined in Java and
cannot be here.

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
