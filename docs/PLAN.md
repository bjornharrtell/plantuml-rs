# Porting plan: PlantUML (Java) to Rust/wasm

Goal: render PlantUML diagrams to SVG in the browser (`wasm32-unknown-unknown`) with no backend,
as a faithful port of [plantuml/plantuml](https://github.com/plantuml/plantuml).

## Decisions

| Topic | Decision |
|---|---|
| Approach | Faithful port of the Java code, not a re-implementation |
| Fidelity | Visually identical to Java's SVG; text width differences accepted (no byte-exact goal) |
| Output | SVG only. No PNG, PDF, EPS, AWT, GUI, servers |
| Layout | [`graphviz-pure`](https://github.com/graphviz-pure/graphviz-pure) (pure-Rust Graphviz 16.1.0 port, wasm-capable, EPL-2.0). Pin the version. Java's Smetana is not ported |
| Text metrics | Embedded font metrics only (deterministic, no browser canvas) |
| License | MIT (PlantUML is multi-licensed and offers MIT) |
| First vertical slice | Class diagram (proves the Graphviz path early) |
| Regex | `fancy-regex` (Java patterns use lookaround); port `ubrex` only if needed |
| Logging | `log` facade only (see `AGENTS.md`) |

## Source analysis (Java, `/home/bjorn/code/plantuml`)

- 3,284 files, 447k lines. Largest: `klimt` 48k (drawing, colours, creole, fonts), `activitydiagram3` 33k,
  `sequencediagram` 25k, `svek` 23k (Graphviz glue), `gantt` 20k, `tim` 14k (preprocessor),
  `command` 11k, `skin` 9k, `style` 6k, Smetana (`gen`/`h`/`smetana`) ~50k (not ported).
- Drawing goes through one seam (`klimt` `UGraphic`/`UDriver`); the SVG backend is `klimt/drawing/svg`.
- Diagram types without Graphviz: sequence, activity-beta, gantt, timing, mindmap, wbs, json, yaml,
  salt, chart, packet, ebnf, nwdiag, etc.
- Graphviz types (via `svek`): class, object, component/deployment, state, use case, legacy activity.
  Java builds a DOT string, runs `dot`, and parses the resulting SVG. We do the same with `graphviz-pure`.
- Existing Java wasm build (TeaVM, `src/main/java/.../teavm`) is the baseline to beat on size and speed.
- Oracle: `src/test/resources/vega` has 472 `.puml` files, 431 with a reference `.svg`
  (`data-diagram-type` in the SVG gives the diagram type).

## Prior art evaluated

- `graphviz-pure`: adopted. New (Oct 2026), single squashed commit, claims byte-identical output to C Graphviz on
  its corpora. Smoke-tested only; validate on real PlantUML-generated DOT early (phase 3).
- `alliecatowo/puml`: not adopted. Independent re-implementation (own AST/renderer/layout), "partial" compatibility.
  Over the Vega fixtures: 89% render without error, label overlap with Java 0.41, sizes within 20% of Java for
  ~0-8% per type; different styling and layout. Useful only as a reference for wasm/LSP API shape.
- Dagre/Sugiyama crates (`dagre`, `rust-sugiyama`, `graphlay`): rejected, output would not match Graphviz.
- `elkrs` (ELK port): candidate later for PlantUML's optional ELK layout mode.

## Architecture

Cargo workspace:

- `plantuml-core`: pure library, no I/O. `render(source, options) -> Result<String /*svg*/, Error>`.
  Inputs: source text, an include resolver trait, a font-metrics provider.
- `plantuml-wasm`: `wasm-bindgen` bindings over `plantuml-core`; npm package.
- `plantuml-cli`: file/stdin to SVG, used by the fidelity harness.

Module layout mirrors the Java packages (`klimt`, `style`, `skin`, `command`, `tim`, `svek`, one module per
diagram type) so that each Rust file maps to a Java file and can be reviewed against it.

## Phases

1. **Scaffold**: workspace, CI (fmt, clippy, tests, `wasm32-unknown-unknown` build), coverage gate (80% patch).
2. **Fidelity harness**: `scripts/fidelity.py <renderer>` runs the renderer (diagram on stdin, SVG on stdout) over
   the Vega fixtures (`VEGA_DIR`) and reports, per diagram type, render success, label overlap and size ratio against
   Java's expected SVG. Later add pixel diff. Commit a baseline.
3. **Foundations**: line/block splitter, preprocessor (`tim`: variables, functions, `!include`, `!theme`),
   colours, skinparam/style cascade, creole text, `klimt` shapes + SVG writer, font metrics trait and embedded fonts.
   Spike: feed real Java-generated DOT through `graphviz-pure` and compare node/edge geometry with Java's.
4. **Class diagram slice**: parser commands, entities, `svek` DOT generation and SVG-result parsing, edge decoration.
5. **Other Graphviz types**: object, component/deployment, state, use case.
6. **Self-layout types**, in order of use: sequence, activity-beta, mindmap, wbs, json/yaml, gantt, salt, timing, chart, packet, others.
7. **wasm packaging**: size budget, npm package, demo page, size/speed comparison against the TeaVM build.

Exit criterion per diagram type: fidelity harness numbers, plus fixtures reviewed visually side by side.

## Risks

- Scale: the Java code is very large; rely on the harness and strict Java-file mapping to keep it tractable.
- `graphviz-pure` maturity; mitigated by pinning and the phase 3 spike. Fallback: Viz.js called from JS.
- Text metrics: Java measures with AWT fonts, so widths will differ slightly; layout drift is expected.
- Preprocessor and standard library (9 MB in Java): wasm needs on-demand loading of stdlib includes.
- Regex semantic differences between Java and Rust engines.
- Licensing: ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0 (dependency).

## Open questions

- Preferred embedded font(s) and their license (candidates: Liberation Sans/Mono, DejaVu Sans).
- Whether the standard library should be bundled, fetched lazily, or left to the host.
- Crate/npm names and publishing.
