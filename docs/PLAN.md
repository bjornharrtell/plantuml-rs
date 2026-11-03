# plantuml-rs: design, fidelity and status

Goal: render PlantUML diagrams to SVG in the browser (`wasm32-unknown-unknown`) with no backend, as a faithful port of
[plantuml/plantuml](https://github.com/plantuml/plantuml).

Base version of the port: PlantUML `1.2026.10beta1`, upstream commit `e6c3adc2be7a50daa5f04b2b0f9d24928aa08efa`
(2026-10-24). The fixtures are from the same checkout.

## Decisions

| Topic | Decision |
|---|---|
| Approach | Faithful port of the Java code by default, so behaviour matches the Vega fixtures. Re-implement idiomatically only where the Java is a workaround for Java or its dependencies (Graphviz process and SVG round trip, TeaVM/Smetana/Elk paths, legacy strategies, AWT/image plumbing). Judge by behaviour and the fixtures, not by the Java structure |
| Fidelity | Same diagram, not same file (see [Fidelity target](#fidelity-target)) |
| Output | SVG only. No PNG, PDF, EPS, AWT, GUI or servers |
| Layout | [`graphviz-pure`](https://github.com/graphviz-pure/graphviz-pure), a pure-Rust Graphviz 16.1.0 port (EPL-2.0), pinned at `=1.0.0`. Java's Smetana is not ported |
| Text metrics | Embedded font metrics only (deterministic, no browser canvas) |
| License | MIT (PlantUML is multi-licensed and offers MIT) |
| C4 models | Through the bundled C4-PlantUML macro library (`!include <C4/C4_Container>`), which expands to description-diagram statements; there is no diagram type of its own |
| Regex | `fancy-regex` (Java patterns use lookaround) |
| Logging | `log` facade only (see `AGENTS.md`) |

## Architecture

Cargo workspace:

- `plantuml-core`: pure library, no I/O. `render(source) -> Result<String, Error>`; includes go through a resolver
  trait (`BundledResolver` in wasm, a filesystem resolver in the CLI).
- `plantuml-wasm`: `wasm-bindgen` bindings; the npm package and the playground are built from it.
- `plantuml-cli`: files or stdin to SVG; also used by the fidelity harness.

The module layout mirrors the Java packages (`klimt`, `style`, `skin`, `command`, `tim`, `svek`, one module per
diagram type) so each Rust file maps to a Java file and can be reviewed against it. Drawing goes through one seam
(`UGraphic`); the SVG writer is `klimt/drawing`. Graphviz-backed types (class, object, description, state) build a DOT
string, lay it out with `graphviz-pure` and read the result back, as Java does with `dot`; `svek` must not depend on
the order of elements in that SVG.

Tooling: `scripts/fidelity.py` with `scripts/svgdiff.py` scores the corpus, `scripts/bench.py` measures speed
([PERFORMANCE.md](PERFORMANCE.md)), `scripts/build-site.sh` builds the playground (with an opt-in browser over every
fixture, `scripts/build-examples.py`), `scripts/build-npm.sh` the npm package.

## Fidelity target

The goal is the same diagram, not the same file. A ported type is accepted when a reader cannot tell the two renderings
apart at a glance and the harness scores agree. Byte, element and pixel identity are not goals; do not tune code to
close text-metric gaps.

Must match:

- content: the same entities, members, labels, stereotypes, links and decorations, with the same text;
- arrangement: the same relative layout, group nesting, reading order and label placement;
- styling: the same colours, line styles, font family and weights, and font sizes.

Accepted differences, not defects: text metrics and glyph shapes (Java measures with AWT fonts, the port with embedded
metrics, so widths, boxes and label positions drift), absolute coordinates, canvas size, element order, attribute
formatting and decimal rounding.

Measured by `scripts/fidelity.py`:

| Metric | Meaning | Target |
|---|---|---|
| `ok%` | renderer exits 0 with an `<svg` document | 100%, or a documented exception list |
| `text_jaccard` | overlap of the text contents with Java's SVG | >= 0.95 |
| `layout_mean` | share of reference elements with the same tag, class and text within 3% of the canvas diagonal | >= 0.90 |
| `identical%` | full structural diff at 0.5 px | diagnostic, never a gate; no longer quoted (see below) |

Fixtures using `!pragma layout smetana` cannot match in arrangement by construction and are excluded and counted
separately.

## Measured fidelity

Scored against the stored corpus (the 472 `.puml` files of `src/test/resources/vega`, 431 with a reference SVG, plus the
repository's own `fixtures/`). `pass` counts scored fixtures; `accepted` counts those declared under
[Accepted differences](#accepted-differences), which are rendered and measured but not held to the target.

| type | n | ok% | text jaccard | layout | pass | accepted |
|---|---|---|---|---|---|---|
| SEQUENCE | 149 | 100% | 0.99 | 0.94 | 134/134 | 15 |
| PACKET | 57 | 100% | 1.00 | 1.00 | 57/57 | 0 |
| CLASS | 53 | 100% | 1.00 | 0.94 | 42/42 | 11 |
| STATE | 48 | 100% | 1.00 | 0.94 | 42/42 | 6 |
| ACTIVITY | 36 | 100% | 0.93 | 0.86 | 24/24 | 12 |
| CHART | 35 | 100% | 1.00 | 1.00 | 35/35 | 0 |
| GANTT | 16 | 100% | 1.00 | 0.96 | 15/15 | 1 |
| SALT | 10 | 100% | 1.00 | 0.95 | 8/8 | 2 |
| WBS | 7 | 100% | 1.00 | 0.95 | 6/6 | 1 |
| DESCRIPTION | 9 | 100% | 1.00 | 0.71 | 4/6 | 3 |
| EBNF | 8 | 100% | 1.00 | 0.95 | 6/8 | 0 |
| MINDMAP | 4 | 100% | 1.00 | 1.00 | 4/4 | 0 |
| CHEN_EER | 4 | 100% | 1.00 | 0.88 | 2/2 | 2 |
| TIMING | 3 | 100% | 1.00 | 1.00 | 3/3 | 0 |
| BPM (crash) | 1 | 100% | 1.00 | 0.88 | - | 1 |
| JSON | 1 | 100% | 1.00 | 0.86 | - | 1 |
| YAML | 1 | 100% | 1.00 | 0.48 | - | 1 |

All 442 fixtures render. **382 of 386 scored fixtures pass (99%)**, 56 are accepted differences, `layout_mean` 0.94,
text jaccard 0.99. The four misses are listed under [In-scope layout gaps](#in-scope-layout-gaps).

Reproduce:

```sh
git clone https://github.com/plantuml/plantuml && git -C plantuml checkout e6c3adc
cargo build -p plantuml-cli
VEGA_DIR=plantuml/src/test/resources/vega FIDELITY_JSON=fidelity.json \
  python3 scripts/fidelity.py target/debug/plantuml-cli
```

`identical%` is not quoted: the stored references were laid out with the point-size line and zero-width spaces
(below), so structural identity against them says nothing about the shipped output. The live-Java comparison is the
diagnostic for that.

### Live Java as a second reference

The stored corpus was produced by PlantUML's test mode (`SVG_DETERMINISTIC` and the Smetana drawing flow), which is not
what the jar renders by default. The harness can render its own references with a jar built from the pinned commit:

```sh
# a JRE-only default JDK needs JAVA_HOME pointing at a JDK
(cd ../plantuml && JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 ./gradlew -q jar)
python3 scripts/fidelity.py target/release/plantuml-cli \
  --java="$(command -v java)" --jar=../plantuml/build/libs/plantuml-1.2026.10beta1.jar --jobs=8
```

Each fixture goes through `java -jar … -tsvg -pipe` (with its own directory as working directory for relative
includes) and is scored against that SVG. A jar older than the checkout is not a good reference.

This is a diagnostic, not a gate. All 442 fixtures render on both sides; the port scores 298 of 375 in-scope fixtures
(layout 0.81) against the live jar, versus 382/386 against the stored corpus. Sequence, packet, chart, salt, mindmap,
gantt, description and the small types are close; the distance is concentrated in class and state, where the jar draws
through its Graphviz flow while the port follows the Smetana flow the stored references use. Visible causes:

- the jar draws class icons and other glyphs as paths, the port as text (the stored test-mode references do the same);
- qualified associations (`HashMap [a1] --> [e] V1`) are not drawn at all: the port parses them but only the Graphviz
  flow renders them (`Kal`), so the stored references lack them too;
- the unused `svek::builder` path is not an alternative: it renders an empty canvas.

### Reference metrics

The Vega references were rendered with Java's `SVG_DETERMINISTIC` format (`StringBounderFromWidthTable`): ordinary and
non-breaking spaces have zero advance, a line is as high as the point size, bold is measured like regular text, and an
embedded `{{ }}` diagram is sized 42 by 42. The shipped output does none of this: spaces keep the bundled face's advance
so words stay apart in a browser, and a line is the face's line box.

`plantuml-cli --deterministic` and `plantuml_core::render_deterministic` reproduce the reference mode, and
`scripts/fidelity.py` uses it for the Vega corpus (not for the repository fixtures, which come from the real jar).
That is what scores the zero-space, point-size-line, deprecated-colour and embedded-diagram fixtures.

### Line box and bold

The line box (ascent + descent + line gap, as Java's `FileFormat.getJavaDimension`) is the height of a text line for
every shipped bounder, so boxes keep the height of the shipped render.

Bold text is measured with the same widths as regular text, as the Vega references do. Java's real bold face is about
1.27x wider, so EBNF rule names and C4 captions come out narrow and several-production grammars stack tighter than
Java's. Using the embedded bold tables fixes `ebnf/grammar` but costs five Vega fixtures (sequence and timing), so it is
not done.

The `fixtures/` references (EBNF, C4) are rendered by the matching jar with `skinparam defaultFontName "Liberation Sans"`
(see `fixtures/README.md`), the face the port measures.

## Accepted differences

Declared in `scripts/accepted-fidelity.txt` (one glob per line, `#` starts the reason) or detected by content; the
harness reports them as `accepted` instead of failures.

| Set | n | Why |
|---|---|---|
| Residual layout deltas (class, state, Chen, description) | 10 | element spacing a few percent off where Java mixes AWT text metrics with its own iteration order: two leaves in a package swapped, a state composite 8 px narrow, Chen entity placement (9% of the canvas), description ports |
| Description rank spacing | 2 | the references were laid out by the Smetana flow (18/36 px spacing); the port follows the Graphviz flow Java ships (35/60 px) |
| `!pragma layout smetana` | 2 of 38 | non-Graphviz layout engine; the other 36 pass |
| Legacy activity syntax (`A0004`) | 1 | ported (links, `if`/`else`/`endif`, partitions, bars); the Smetana reference places the end node and the second branch differently (layout 0.75) |
| `scale` on an embedded diagram (`tikz-scale-down`, `tikz-scale-up`) | 2 | the scale transform of embedded images is not ported |
| Handwritten mode | 1 | needs a handwritten font that is not embedded |

## In-scope layout gaps

Four repository fixtures miss the layout target. They match on text and content; the cause is bold width and, for C4,
boundary and caption layout:

| Fixture | layout | Cause |
|---|---|---|
| `fixtures/c4/context` | 0.77 | boundary and caption layout, narrow bold |
| `fixtures/c4/container` | 0.84 | same, with a boundary around two containers |
| `fixtures/ebnf/grammar` | 0.86 | bold rule names are narrower than Java's, so the productions stack tighter |
| `fixtures/ebnf/expression` | 0.74 | same |

Description diagrams keep at least 35 px between nodes and 60 px between ranks, as the Graphviz flow Java ships does;
that scores better against a live Java render than against the Smetana references.

## Activity diagrams

The activity diagram follows Java's tiles (`activitydiagram3/ftile`), connection by connection, and was checked against
a live jar with side-by-side renders and line coordinates.

Ported:

- `if`: `FtileIfWithLinks` (two real branches, merge diamond), `FtileIfDown` (the other branch empty or a lone `stop`,
  with `ConditionalBuilder`'s selection), `FtileIfLongHorizontal` for `elseif`; `switch`
  (`FtileSwitchWithManyLinks` / `OneLink`).
- Loops: `while` (labels, `backward`, `break`), `repeat` (start box, `is (yes) not (no)`, `backward`, `break`).
- Notes: `note left|right`, multi-line, `floating note`, attached to the latest activity; a note on an `if` sits beside
  its condition.
- Swimlanes: `|lane|`, `|#color|lane|label|`, `swimlane name as label`; every lane is measured and drawn in its own
  pass through a filter that keeps that lane's shapes, then one more pass draws the arrows between lanes with the
  tiles' `drawTranslate` variants; titles, backgrounds and dividers.
- Groups and partitions, fork, arrow labels and colours, including the colour of the arrow leaving a branch.

Also ported: notes on an `elseif` chain and on a `repeat` with `backward`, `kill`/`detach`, `label`/`goto`,
`split`/`merge` parallels, and a `while` followed by `stop` or `end` (`specialOut`).

The group frame matches Java: `max(25, title + 20)` above, 20 below, and the frame rectangle is ignored for vertical
compression, so the empty bands inside it shrink exactly as in Java. Not ported: `SpecialText`, the title wrapper that
only matters when compressing on X.

## Foundations not done

- Creole: tables, trees, code blocks, images, emoji, math, and links rendered as `<a>` (a `[[url label]]` becomes plain
  text).
- Stdlib: icon libraries such as `tupadr3`, the cloud providers and the C4 `themes/` folder are not bundled. The C4
  library (seven files, MIT) and 44 themes are.
- Handwritten mode (`!option handwritten`, `skinparam handwritten`).
- C4: 13 of the 17 C4-PlantUML samples render once their URL includes are rewritten to `<c4/...>`; the icon sample, the
  two theme samples and the sequence-mode sample still fail.

## Out of scope

- **Unported diagram types**: `@startbpm`, `@startnwdiag`, `@startchronology`, `@startregex`, `@startwire`,
  `@startboard`, `@startgit`, `@startfiles`, `@startditaa`, `@startdot`, `@startmath`, `@startlatex`, `@startjcckit`,
  `@startflow`/`@startdef`, the help/sprites pages. They return `Unsupported`
  with a clear message. `@startproject` is a Gantt alias and works.
- **Publishing the VS Code extension from CI**: `scripts/build-vscode-ext.sh` builds it; publishing is by hand.

## Risks

- Scale: the Java code is very large; the harness and the strict Java-file mapping keep it tractable.
- `graphviz-pure` maturity: mitigated by the pin and by the corpus scoring any change; fallback is Viz.js from JS.
- Text metrics differ from AWT by construction; accepted, not chased.
- Regex semantics differ between Java and Rust engines (`regex::transform` unescapes `\<` and `\>`).
- Licensing: ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0 (a dependency).
