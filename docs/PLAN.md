# Porting plan: PlantUML (Java) to Rust/wasm

Goal: render PlantUML diagrams to SVG in the browser (`wasm32-unknown-unknown`) with no backend,
as a faithful port of [plantuml/plantuml](https://github.com/plantuml/plantuml).

## Decisions

| Topic | Decision |
|---|---|
| Approach | Faithful port of the Java code by default, so behavior matches the Vega fixtures. Re-implement idiomatically only where the Java is a workaround for Java or its dependencies (Graphviz process and SVG round trip, TeaVM/Smetana/Elk paths, legacy strategies, AWT/image plumbing) or an infrastructure-heavy area (stdlib/theme loading, Creole, activity). Judge by behavior and the fixtures, not by the Java structure |
| Fidelity | Same diagram, not same file: content, arrangement and styling match Java; text metrics, glyph shapes and exact coordinates do not. See [Fidelity target](#fidelity-target) |
| Output | SVG only. No PNG, PDF, EPS, AWT, GUI, servers |
| Layout | [`graphviz-pure`](https://github.com/graphviz-pure/graphviz-pure) (pure-Rust Graphviz 16.1.0 port, wasm-capable, EPL-2.0). Pin the version. Java's Smetana is not ported |
| Text metrics | Embedded font metrics only (deterministic, no browser canvas) |
| License | MIT (PlantUML is multi-licensed and offers MIT) |
| First vertical slice | Class diagram (proves the Graphviz path early) |
| C4 models | A desired supported type, and not a diagram type of its own: C4-PlantUML is a stdlib macro library (`!include <C4/C4_Container>`) that expands to DESCRIPTION (component) statements. Supporting C4 therefore means bundled stdlib includes plus the DESCRIPTION diagram type |
| Regex | `fancy-regex` (Java patterns use lookaround); port `ubrex` only if needed |
| Logging | `log` facade only (see `AGENTS.md`) |

## Fidelity target

The goal is the same diagram, not the same file. A ported type is accepted when a reader cannot tell the two
renderings apart at a glance and the harness scores below agree. Byte, element and pixel identity are explicitly
not goals; do not tune code to close text-metric gaps.

Must match:

- content: the same entities, members, labels, stereotypes, links and decorations, with the same text;
- arrangement: the same relative layout, group nesting, reading order and label placement;
- styling: the same colours, line styles, font family/weights and font sizes.

Accepted differences, not defects:

- text metrics and glyph shapes: Java measures strings with AWT fonts, the port uses embedded metrics
  (`klimt/font`), so widths differ and canvas sizes, boxes and label positions drift with them;
- absolute coordinates, canvas size, element order and attribute formatting in the SVG;
- whitespace inside generated paths, and decimal rounding.

Measured, not eyeballed (`scripts/fidelity.py`):

| Metric | Meaning | Target |
|---|---|---|
| `ok%` | renderer exits 0 with an `<svg` document | 100%, or a documented per-type exception list |
| `text_jaccard` | overlap of the text contents with Java's SVG | >= 0.95 |
| `layout_mean` | share of reference elements that have the same tag, class and text within 3% of the canvas diagonal (`svgdiff.layout`, `LAYOUT_TOL`) | >= 0.90 |
| `size_within20%` | width and height ratio | informational: font metrics alone move it |
| `identical%` | full structural diff at 0.5 px (`svgdiff.differences`, `SVG_TOL`) | diagnostic only, never a gate |
| visual review | the type's fixtures side by side with Java's SVG | sign off and record the accepted differences |

Known exceptions to the layout target: fixtures using `!pragma layout smetana` (or another non-Graphviz layout mode)
cannot match in arrangement by construction; they are excluded and listed per type.

## Source analysis (Java, `/home/bjorn/code/plantuml`)

Base version of the port (as of 2026-10-25): PlantUML `1.2026.10beta1`, upstream commit `e6c3adc2be7a50daa5f04b2b0f9d24928aa08efa` (2026-10-24, "Vega migration: convert two tests left in the old JUnit/Java format"). Fixtures are from the same checkout.


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
  its corpora. Validated on 72 real PlantUML DOT inputs (see Spike).
- `alliecatowo/puml`: not adopted. Independent re-implementation (own AST/renderer/layout), "partial" compatibility.
  Over the Vega fixtures: 89% render without error, label overlap with Java 0.41, sizes within 20% of Java for
  ~0-8% per type; different styling and layout. Useful only as a reference for wasm/LSP API shape.
- Dagre/Sugiyama crates (`dagre`, `rust-sugiyama`, `graphlay`): rejected, output would not match Graphviz.
- `elkrs` (ELK port): candidate later for PlantUML's optional ELK layout mode.

## Spike: `graphviz-pure` on real PlantUML DOT (done)

Captured the DOT that Java PlantUML sends to `dot` for the 72 Vega fixtures that use Graphviz (class, state,
component, object; `scripts/dot-capture.sh` is a `GRAPHVIZ_DOT` wrapper that records it) and ran each through
`graphviz-pure` 0.2 (Graphviz 16.1.0 port) and system `dot` 14.1.2 (`-Tsvg`).

- 72/72 render without error; 39 are byte-identical to C `dot` (after dropping the header comments).
- The other 33 contain the same set of SVG lines, only in a different order (cluster elements are emitted in
  another order; likely a Graphviz 14 vs 16 difference). Geometry is identical.
- Speed: 72 graphs in 30 ms total.
- Native `graphviz-pure` prints Graphviz warnings to stderr; the wasm build must route them to `log`.

Conclusion: `graphviz-pure` is good enough to adopt. `svek` must not depend on the order of SVG elements.

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
   the Vega fixtures (`VEGA_DIR`) and reports, per diagram type: render success, text overlap, relative element
   placement (`layout_mean`), size ratio and the structural diff count. Acceptance is defined in
   [Fidelity target](#fidelity-target); pixel and byte identity are not goals.
3. **Foundations**: line/block splitter, preprocessor (`tim`: variables, functions, `!include`, `!theme`),
   colours, skinparam/style cascade, creole text, `klimt` shapes + SVG writer, font metrics trait and embedded fonts.
   (The `graphviz-pure` spike is done, see above.)
4. **Class diagram slice**: parser commands, entities, `svek` DOT generation and SVG-result parsing, edge decoration.
5. **Other Graphviz types**: object, component/deployment (DESCRIPTION), state, use case.
6. **C4 models**: bundle C4-PlantUML sources, resolve `!include <C4/...>` and render the canonical C4 samples (see the
   C4 section below). Depends on phase 5 for DESCRIPTION.
7. **Self-layout types**, in order of use: sequence, activity-beta, mindmap, wbs, json/yaml, gantt, salt, timing, chart, packet, others.
8. **wasm packaging**: size budget, npm package, demo page, size/speed comparison against the TeaVM build.

Exit criterion per diagram type: `ok%` at 100% (or a documented exception list), `text_jaccard` >= 0.95 and
`layout_mean` >= 0.90 on that type's Vega fixtures, plus a side-by-side review that records the accepted differences.
`identical%` is reported for regression tracking but is not a gate.

## C4 model support

C4 has no `@startc4`; diagrams are plain `@startuml` sources that include the C4-PlantUML macro library and call its
procedures (`Person`, `System`, `Container`, `Component`, `*_Boundary`, `Rel`). The macros emit DESCRIPTION statements:

- `rectangle "<creole label>" <<person|system|container|component|boundary>> as <alias>`, default shape `rectangle`
  (`$portraitPerson = "false"`); `Person` emits `actor`/`person` only after `SHOW_PERSON_PORTRAIT()` or
  `SHOW_PERSON_OUTLINE()`.
- `Rel()` and friends emit one relationship statement (`-->`, `-->>`, `..>`) with the label, `$techn` and `$descr`
  folded into it; the `Lay_*` helpers add `-[hidden]` ones for layout only.
- Styling is `skinparam rectangle<<stereotype>> { ... }` written by `UpdateElementStyle`/`AddElementTag`, plus
  `LAYOUT_WITH_LEGEND()` (legend), `hide stereotype`, `skinparam shadowing false`.
- Boundaries are titled rectangles (`== label ==` heading), not clusters, in the current C4 style.

What that needs, and where the port stands:

| Piece | State |
|---|---|
| Preprocessor builtins C4 uses (`%substr`, `%strpos`, `%strlen`, `%splitstr`, `%set_variable_value`, `%get_variable_value`, `%variable_exists`, `%function_exists`, `%intval`, `%chr`, `%lower`, `%is_dark`, `%darken`, `%newline`, `%breakline`, `%true`/`%false`), plus `!global`, `!unquoted procedure`, `!function`/`!return`, `!while`, `!foreach`, `!elseif` | present in `tim` |
| `!include <name>` resolution | `read_stdlib` hook exists (`tim/builtin.rs`, `tim/context.rs`) but no bundled data, and `render` passes `NoResolver`, so `<C4/...>` cannot resolve in wasm or the CLI |
| C4 sources | not bundled. Java ships them packed as `.spm` archives (`src/main/resources/stdlib/c4`); take the plain `.puml` sources from `plantuml-stdlib/C4-PlantUML` instead (7 files, 3,361 lines, MIT) |
| DESCRIPTION diagram type | not ported: `component`, `rectangle`, `node`, `database`, `queue`, `cloud`, `person`, `actor`, `interface`, `storage`, `card` all fail to parse, and 0 of the 6 Vega DESCRIPTION fixtures render |
| Stereotype styles, creole headings, legend chrome | present (`style`, `klimt/creole`, `chrome.rs`), so the C4 look is mostly a matter of the DESCRIPTION image |

Exit criterion for C4: the canonical samples (`samples/` in C4-PlantUML: context, container, component, deployment,
dynamic) render, reach `text_jaccard` >= 0.95 and `layout_mean` >= 0.90 against the generated Java references, and are
reviewed side by side; `Person`, `System`, `Container`, `Component`, `*_Boundary`, `Rel` first, then sprites and
`$baseShape` variants.

Oracle: no C4 sources exist in the Vega fixtures (0 of 431 reference C4), so C4 needs generated references: render the
samples with the Java jar built from base commit `e6c3adc` and score them with `scripts/fidelity.py` (or
`scripts/svgdiff.py layout` for one pair).

## C4 model diagrams

C4 (`!include <C4/C4_Container>` etc.) is not a diagram type of its own: the C4-PlantUML library is a set of `!procedure`/`!function` definitions that expand to description/component diagram elements (`rectangle`, `database`, `queue`, `person` stereotypes, `Rel` links, boundaries as packages) with C4 styling through `skinparam`/`<style>`. Support therefore needs, in order:

1. Description/component diagrams (`descdiagram`), including nested boundaries (clusters) and stereotyped rectangles.
2. Stdlib loading for `<C4/...>` includes (`src/main/resources/stdlib/c4`, `.spm` archives). Fetch lazily or bundle only the C4 subset, since the whole stdlib is 9 MB.
3. The preprocessor features the library uses (procedures with default arguments, `!ifndef`, `$argument` expansion, sprites, `%`-builtins), already in `tim`; verify against the library.
4. Fixtures: render the C4 samples and compare with Java's SVG.

Progress is tracked with the description diagram work.

## Risks

- Scale: the Java code is very large; rely on the harness and strict Java-file mapping to keep it tractable.
- `graphviz-pure` maturity; mitigated by pinning and the spike (see above). Fallback: Viz.js called from JS.
- Text metrics: Java measures with AWT fonts, so string widths differ and boxes, canvases and label positions follow.
  This is an accepted difference (see [Fidelity target](#fidelity-target)), not a bug to chase.
- Chasing exact SVG equality: element order, attribute formatting and coordinates differ by construction; `identical%`
  is a regression signal only.
- Preprocessor and standard library (9 MB in Java): wasm needs on-demand loading of stdlib includes.
- Regex semantic differences between Java and Rust engines.
- Licensing: ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0 (dependency).

## Open questions

- Preferred embedded font(s) and their license (candidates: Liberation Sans/Mono, DejaVu Sans).
- Whether the standard library should be bundled, fetched lazily, or left to the host.
- C4: bundle only the C4-PlantUML sources or a wider stdlib slice; whether sprites (`tupadr3/font-awesome`) and the C4
  themes (`themes/puml-theme-C4_*.puml`, a `!theme`-shaped feature) are in scope for the first C4 slice; whether
  `C4_Sequence`/`C4_Dynamic` reuse the sequence renderer or need their own handling.
- Crate/npm names and publishing.

## Effort estimate

- Full parity: ~150k lines of Rust, 9-12 months of focused work.
- Useful subset (class, sequence, activity, state, component, usecase): 3-4 months.
- Class-diagram vertical slice: 4-6 weeks.

## Progress

Fixture counts in this log are historical. "Identical" means the exact-structure diagnostic matched byte for byte at the
time; it is a regression signal, not the acceptance bar (see [Fidelity target](#fidelity-target)).

- Done: colors, embedded font metrics (PlantUML's own Liberation Sans tables, `metrics.bin`), XML writer, SVG backend (`klimt/drawing`).
- Done: `UGraphic` context, shapes (rect, line, ellipse, polygon, path, text) and SVG drivers (`klimt/drawing/ugraphic.rs`).
- Done: style system (parser, cascade, index, built-in `plantuml.skin`) in `style/`.
- Done: text blocks, horizontal-line stencils and Creole text (styles, colors, lists, headings, hlines, line breaking) in `klimt/shape.rs` and `klimt/creole/`. Not yet: tables, trees, code blocks, images, sprites, emoji, math, links as `<a>`.
- Done: composable regex (`regex.rs`), source lines (`text.rs`), command framework and parse loop (`command.rs`).
- Done: skinparam → style conversion (`style/skinparam.rs`) and `SkinParam` settings (`skin/mod.rs`).
- Next: creole, preprocessor, class parser, svek.
- Done: self messages (`ComponentRoseSelfArrow`, `CommunicationTileSelf`), byte-identical to Java on reference cases.
- Done: groups (alt/loop/opt/par/group/critical/break/partition): `GroupingTile`, `ElseTile`, `Blotter`, rose grouping components, `CommandGrouping`. UGraphic default colors are now transparent like Java.
- Done: notes (`NoteTile`, `NotesTile`, note-on-message tiles, group end notes, rose note components, `BodyEnhanced`, note commands) and parallel sibling disjoint constraints. Vega SEQUENCE: 45/109 identical.
- Done: participant figures (actor styles, boundary, control, entity, database, queue). Vega SEQUENCE: 57/109.
- Done: participant boxes (`Dolls`, englober), `ref over` (`ReferenceTile`), `StringUtils` helpers. Vega SEQUENCE: 73/109.
- Done: JSON, YAML and HCL data diagrams (`jsondiagram/`, `yaml/`, `hcl/`): value parsers, key/value boxes (`TextBlockJson`), links laid out with `graphviz-pure` on fixed-size HTML-table nodes (stand-in for the patched Smetana records); `skinparam handwritten` not drawn. Vega JSON 1/1 matches in structure (x drift from text widths); YAML fixture needs `!theme` resolution in the CLI.
- Done: border (exo) messages (`CommunicationExoTile`, exo arrow commands). Vega SEQUENCE: 75/109.
- Done: diagram chrome (title, header, footer, legend, caption, mainframe, warnings banner): `chrome.rs`, `DecorateEntityImage`. Vega SEQUENCE: 77/109.
- Done: preprocessor (`tim`: comments, variables, expressions, functions/procedures, conditionals, loops, includes, subs, options, builtins), source block splitting (`preproc.rs`), `json.rs`, HSLuv. Vega SEQUENCE: 101/109.
- Done: diagram registry (`psystem.rs`), packet diagrams (56/56 Vega fixtures identical), `skin` command and bundled skins.
- Done: wasm package and playground (`site/`, `scripts/build-site.sh`, `.github/workflows/pages.yml`; enable Pages with source "GitHub Actions" in the repository settings).
- Done: chart diagrams (`chart/`: axes, bar/line/area/scatter series, legend, annotations). Vega CHART: 35/35 identical.
- Done: salt diagrams (`salt/`: tokenizer, element factories, tables, trees, tabs, menus, droplists, scroll, border, images; sprites skipped). Vega SALT: 6/10 identical, 4 differ only by line order (Java HashSet) or by creole `[[url{tooltip}]]` links.
- In progress: class diagrams. Ported: `abel` model, `plasma`, `cucadiagram`, link decorations, visibility icons, class/link commands, `set separator`/`namespaceSeparator`, `hide`/`show` (by name, stereotype, gender, visibility), `left to right`/`top to bottom direction`, `package`/`namespace` blocks with `}`, `DotPath`, `svek` (types, node, role labels, extremities, bibliotekon, cluster), `SvekEdge`, `DotStringFactory` on graphviz-pure, `EntityImageClass`. Next: cluster layout and drawing for `package`/`namespace` bodies (`Cluster::print_internal` and the cluster box), then notes on members and association classes.
- Done: mindmap and WBS diagrams (`mindmap/`, `wbs/`: tetris packing, fingers, forks, links). Vega MINDMAP 4/4, WBS 7/7 identical.
- Done: state diagrams (`statediagram/`): state parser and commands (composite/concurrent regions, descriptions, history/fork/join pseudo-states, `hide $x`), model registered as `StateDiagramFactory`, layout on `sdot` with autarkic composite pre-rendering (`InnerStateAutonom`, `ConcurrentStates`, `PaddedEntityImage`), rounded state cluster headers (`RoundedContainer`). Vega STATE: 48/48 render, 16 identical, 33% exact structure.
- Done: gantt diagrams (`gantt/`): tasks, dependencies, milestones, resources, calendars, colors. Vega GANTT: 16/16 render, 15 identical.
- Done: timing diagrams (`timingdiagram/`): clocks, binary/bit signals, constraints, scales, highlights. Vega TIMING: 3/3 identical.
- In progress: activity diagrams (`activitydiagram3/`): instruction tree, ftile layer (box, start/stop/end, assembly, arrows with `Snake`/`Worm` merging), `klimt/compress`, commands for actions, arrows, start/stop/end, box styles, stereogroup colors. Vega ACTIVITY: the plain action/arrow fixtures are identical. Next: if/switch, while/repeat, fork/split, groups/swimlanes, notes, goto/labels, spots, kill.
- Next: description/component diagrams, then C4 models (see the C4 section).

## Measured fidelity (2026-10-26)

graphviz-pure stands in for `dot`. Scored against the targets in [Fidelity target](#fidelity-target);
`!pragma layout smetana` fixtures are excluded from the layout target by construction.

| type | n | ok% | text jaccard | layout | exact structure (diagnostic) | meets target |
|---|---|---|---|---|---|---|
| SEQUENCE | 149 | 97% | 0.95 | 0.94 | 93% | yes |
| PACKET | 57 | 100% | 1.00 | 1.00 | 100% | yes |
| CHART | 35 | 100% | 1.00 | 1.00 | 100% | yes |
| CLASS | 53 | 92% | 0.90 | 0.83 | 49% | no (layout) |
| STATE | 48 | 100% | 1.00 | 0.95 | 33% | yes |
| ACTIVITY | 36 | 22% | 0.18 | 0.19 | 17% | no |
| GANTT | 16 | 100% | 1.00 | 1.00 | 94% | yes |
| SALT | 10 | 100% | 0.96 | 0.99 | 60% | yes |
| WBS | 7 | 100% | 1.00 | 1.00 | 100% | yes |
| DESCRIPTION | 6 | 17% | 0.08 | 0.00 | 0% | no |
| MINDMAP | 4 | 100% | 1.00 | 1.00 | 100% | yes |
| CHEN_EER | 4 | 0% | - | 0.00 | 0% | no |
| TIMING | 3 | 100% | 1.00 | 1.00 | 100% | yes |
| BPM | 1 | 0% | - | 0.00 | 0% | no |
| JSON | 1 | 100% | 1.00 | 0.92 | 0% | yes |
| YAML | 1 | 100% | 1.00 | 0.90 | 0% | yes |

All 431 fixtures: 89% render, `layout_mean` 0.86, text jaccard 0.88, 313 exact structures. Sequence, packet, chart,
state, gantt, timing, mindmap, wbs, salt and json/yaml meet the target. CLASS renders 49 of 53 and is 0.83 on layout
(association classes, notes on members). ACTIVITY, DESCRIPTION and CHEN_EER are the large remaining gaps.

`size_within20%` is reported per fixture in the JSON but not tabulated here: it moves with font metrics and is not a
signal of correctness.

Reproduce:

```sh
git clone https://github.com/plantuml/plantuml && git -C plantuml checkout e6c3adc
cargo build -p plantuml-cli
VEGA_DIR=plantuml/src/test/resources/vega FIDELITY_JSON=fidelity.json \
  python3 scripts/fidelity.py target/debug/plantuml-cli
```
- Activity slices still to do: if/switch, while/repeat, fork/split, groups/partitions/swimlanes, notes, goto. Also open: SVG `<a>` hyperlinks, description/object diagrams (and C4 on top), EBNF/regex/bpm/wire/board/git/files, `!import` zip, handwritten mode.
