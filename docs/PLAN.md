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
| Layout | [`graphviz-pure`](https://github.com/graphviz-pure/graphviz-pure), a pure-Rust Graphviz 16.1.0 port (EPL-2.0), pinned at `=1.1.0`. Java's Smetana is not ported |
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
| `identical%` | full structural diff at 0.5 px | diagnostic, never a gate |

Fixtures using `!pragma layout smetana`, and the few whose reference was laid out by Smetana or uses the legacy
activity syntax, are out of scope: they are not scored and not counted (`scripts/out-of-scope-fixtures.txt`).

## Measured fidelity

Scored against the stored corpus (the `.puml` files of `src/test/resources/vega` with a reference SVG, plus the
repository's own `fixtures/`), in the reference metrics mode below. `pass` counts scored fixtures; `accepted` counts those
declared under [Accepted differences](#accepted-differences), which are rendered and measured but not held to the target.

| type | n | ok% | text jaccard | layout | pass | accepted |
|---|---|---|---|---|---|---|
| SEQUENCE | 149 | 100% | 1.00 | 0.99 | 146/146 | 3 |
| PACKET | 57 | 100% | 1.00 | 1.00 | 57/57 | 0 |
| CLASS | 36 | 100% | 1.00 | 0.94 | 32/32 | 4 |
| ACTIVITY | 35 | 100% | 1.00 | 1.00 | 35/35 | 0 |
| CHART | 35 | 100% | 1.00 | 1.00 | 35/35 | 0 |
| STATE | 27 | 100% | 1.00 | 0.94 | 24/24 | 3 |
| GANTT | 16 | 100% | 1.00 | 1.00 | 16/16 | 0 |
| SALT | 10 | 100% | 1.00 | 1.00 | 10/10 | 0 |
| EBNF | 8 | 100% | 1.00 | 1.00 | 8/8 | 0 |
| WBS | 7 | 100% | 1.00 | 1.00 | 7/7 | 0 |
| DESCRIPTION | 7 | 100% | 1.00 | 0.85 | 6/6 | 1 |
| CHEN_EER, MINDMAP, TIMING, BPM, JSON, YAML | 14 | 100% | 1.00 | 1.00 | 14/14 | 0 |

All 401 scored fixtures render; **390 of 390 in-scope fixtures pass**, 11 are accepted differences, `layout_mean` 0.98.

Reproduce:

```sh
git clone https://github.com/plantuml/plantuml && git -C plantuml checkout e6c3adc
cargo build --release -p plantuml-cli
VEGA_DIR=plantuml/src/test/resources/vega FIDELITY_JSON=fidelity.json \
  python3 scripts/fidelity.py target/release/plantuml-cli
```

### Live Java as a second reference

The harness can also render its own references with a jar built from the pinned commit. This is a diagnostic, not a
gate: the stored corpus comes from Java's test mode, which is not what the jar renders by default.

```sh
# a JRE-only default JDK needs JAVA_HOME pointing at a JDK
(cd ../plantuml && JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 ./gradlew -q jar)
python3 scripts/fidelity.py target/release/plantuml-cli \
  --java="$(command -v java)" --jar=../plantuml/build/libs/plantuml-1.2026.10beta1.jar --jobs=8
```

Each fixture goes through `java -jar … -tsvg -pipe` and is scored against that SVG. A jar older than the checkout is
not a good reference. Class and state score lowest there, because the jar draws them through its Graphviz flow.

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
every shipped bounder. Bold text is measured with regular widths, as the Vega references do; Java's real bold face is
about 1.27x wider, so EBNF rule names and C4 captions come out narrow. The `fixtures/` references (EBNF, C4) are
rendered by the matching jar with `skinparam defaultFontName "Liberation Sans"` (see `fixtures/README.md`), the face
the port measures.

## Accepted differences

Declared in `scripts/accepted-fidelity.txt` (one glob per line, `#` starts the reason) or detected by content; the
harness reports them as `accepted` instead of failures.

| Set | n | Why |
|---|---|---|
| Residual layout deltas (class, state, Chen, description) | 10 | element spacing a few percent off where Java mixes AWT text metrics with its own iteration order: two leaves in a package swapped, a state composite 8 px narrow, Chen entity placement (9% of the canvas), description ports |
| `scale` on an embedded diagram (`tikz-scale-down`, `tikz-scale-up`) | 2 | the scale transform of embedded images is not ported |
| Handwritten mode | 1 | needs a handwritten font that is not embedded |

Description diagrams keep at least 35 px between nodes and 60 px between ranks, as the Graphviz flow Java ships does.

## Activity diagrams

The activity diagram follows Java's tiles (`activitydiagram3/ftile`), connection by connection.

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

Also: notes on an `elseif` chain and on a `repeat` with `backward`, `kill`/`detach`, `label`/`goto`,
`split`/`merge` parallels, and a `while` followed by `stop` or `end` (`specialOut`).

The group frame matches Java: `max(25, title + 20)` above, 20 below, and the frame rectangle is ignored for vertical
compression, so the empty bands inside it shrink exactly as in Java.

## Foundations not done

- Creole: tables, trees, code blocks, images, emoji, math, and links rendered as `<a>` (a `[[url label]]` becomes plain
  text).
- Stdlib: icon libraries such as `tupadr3`, the cloud providers and the C4 `themes/` folder are not bundled. The C4
  library (seven files, MIT) and 44 themes are.
- Handwritten mode (`!option handwritten`, `skinparam handwritten`).
- C4: 13 of the 17 C4-PlantUML samples render once their URL includes are rewritten to `<c4/...>`; the icon sample, the
  two theme samples and the sequence-mode sample still fail.

## Out of scope

- **Smetana-dependent layout and the legacy (pre-beta) activity syntax in the fixtures**: `!pragma layout smetana`
  fixtures and the three listed in `scripts/out-of-scope-fixtures.txt` are not scored. The legacy activity syntax is
  still rendered.
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
