# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the API is unstable until 1.0.

## [Unreleased]

## [Unreleased]

### Changed

- `graphviz-pure` 1.1.0 (was 1.0.0). Output of the Vega corpus is byte-identical and speed is unchanged.

### Fixed

- Later `skinparam` values now override earlier ones for the same stereotype, so C4 `UpdateElementStyle` and tag colours take effect.

## [0.5.3] - 2026-11-03

### Added

- Legacy activity diagrams (`(*) --> a`, `if`/`else`/`endif`, partitions, synchronisation bars) and `{{ }}` embedded
  diagrams in labels, drawn as SVG images.
- Activity diagrams: `kill`/`detach`, `label`/`goto`, `split`/`merge`, and `stop`/`end` right after `endwhile`.
- `plantuml-cli --deterministic` and `render_deterministic`: the metrics of Java's `SVG_DETERMINISTIC` format, for
  comparing with its reference corpus.

### Fixed

- Group and partition frames are sized and squeezed as in Java; wide frame titles are drawn as `SpecialText`.

## [0.5.2] - 2026-11-02

### Fixed

- Class diagrams: link multiplicities and other head/tail labels no longer land inside node boxes; the Smetana
  drawing path now moves them away from the nodes as Java does (`manageCollision`).
- npm: 0.5.1 was published to crates.io and GitHub only; its npm release failed on a duplicate provenance entry, so
  0.5.2 is the first 0.5 release on npm.

## [0.5.1] - 2026-11-01

### Changed

- The release workflow publishes the npm package `plantuml-rs` with npm trusted publishing (OIDC, no token), after
  the crates.

## [0.5.0] - 2026-11-01

### Added

- Activity swimlanes: `|lane|`, `|#color|lane|label|` and `swimlane name as label`, which were syntax errors. Each lane
  is drawn in its own pass with its title, background colour and dividers, and the arrows that leave a lane are
  redrawn between the lanes for `if`, `elseif`, `switch`, `while`, `repeat` and `fork`, as Java does.
  `swimlaneWidth`, `swimlaneWrapTitleWidth` and the `swimlane` style apply.
- Activity notes: `note left|right: text`, multi-line `note ... end note` and `floating note`, which were syntax
  errors. A note belongs to the latest activity (inside a branch, to the activity before it); a note after an `if`
  sits beside its condition.
- Activity `break`, and `backward :text;` in `while` and `repeat` loops; all were syntax errors.
- Playground: an opt-in "all fixtures" checkbox opens a browser over every repository fixture and every file of the
  PlantUML test corpus, with a filter and previous/next buttons. The default view stays one example per diagram type.
  `scripts/build-examples.py` writes `site/examples.json`; the Pages workflow fetches the pinned corpus for it.

### Fixed

- Activity tiles follow Java's: `if` (two real branches keep the merge diamond; an `if` without `else`, or with a
  `break` or `stop`, goes straight down with the other path routed around the branch), `elseif` (conditions side by
  side, no merge diamond; its condition text was dropped), `switch` (cases in a row with labels on the arrows),
  `while` (loop-back and exit arrows no longer overlap; `is (yes)` / `endwhile (no)` labels were dropped) and
  `repeat` (condition below, `is (yes) not (no)` labels, loop-back on the right). The repeat and switch fixtures
  reach layout 1.0 (`repeat_stereo` was 0.0, a three-case `switch` 0.22).
- The arrow leaving a branch takes the colour set after its last activity, not the colour of the arrow into it.

### Changed

- Native release builds use `opt-level = 3`, about 30% faster rendering (corpus 1.22 s to 0.8 s). The wasm build
  moves to a `wasm` profile (`cargo build --profile wasm`) that keeps `opt-level = "z"`; its output is unchanged.
- Regex compilation skips case folding for `\w` and `\p{..}` classes: the first render takes 31 ms (was 38).
- `graphviz-pure` 1.0.0 (was 0.2.2). Its calls return the output with Graphviz diagnostics, which are now logged with
  `log::warn!` instead of reaching stderr. The rendered corpus is unchanged.
- The EBNF and C4 fixture references are re-rendered with Liberation Sans, the face the port measures (they came from a
  machine whose `sans-serif` was the wider DejaVu Sans).

## [0.4.0] - 2026-10-28

### Changed

- Text lines are measured with the font's line box (ascent + descent + line gap) instead of the point
  size, as Java's `FileFormat.getJavaDimension` does. Boxes, rows and canvases reach their original
  height: against a live Java render the corpus goes from layout 0.68 to 0.71 and from 171 to 206
  passing fixtures. Five fixtures whose references were laid out with the shorter line are declared.

### Added

- `plantuml-cli`: several inputs in one run, `-o/--out DIR`, `--check` (validate, non-zero exit) and
  `--json` (one object per input with the diagram type, canvas size, SVG, or a structured error with its
  line). These are the building blocks for scripts and agents; `-tsvg` is accepted for compatibility.
- npm package `plantuml-rs`: the wasm build as a CommonJS/Node build and an ESM/browser build, with types
  (`scripts/build-npm.sh`).
- GitHub releases now attach `plantuml-cli` binaries for Linux x64, macOS x64/arm64 and Windows x64, with
  SHA256 checksums and the changelog section as notes.
- VS Code extension: syntax highlighting and language configuration for `.puml`/`.plantuml`/`.iuml`/`.pu`,
  a **PlantUML: Export SVG** command, and an icon. The Marketplace and Open VSX listings are now linked from
  the README and the playground.

## [0.3.1] - 2026-10-27

### Fixed

- Description diagrams (component, deployment, C4): an empty package or boundary is laid out as a node,
  as Java does, instead of a cluster stacked below the content. It takes its place in the ranks, so C4
  `System_Boundary(x, "y") { }` and the relationships to it land where the original puts them; the C4
  container sample moves from 0.19 to 0.67 against a live Java render.

## [0.3.0] - 2026-10-27

### Changed

- `graphviz-pure` 0.2.2 (was 0.2.1). The release changes only the `neato` and `sfdp` engines; the
  rendered corpus is unchanged.

### Fixed

- A class or object link that names a member (`Foo::field1 --> Bar::field3`) now attaches to that
  member's row and routes around the box, as PlantUML's Graphviz output does, instead of putting both
  ends on the border of the class.
- Description diagrams (component, deployment and C4) keep the Graphviz flow's rank separation
  (35 px between nodes, 60 px between ranks) instead of Graphviz's tighter defaults, so lanes and
  columns are spaced like the original. Large diagrams grow accordingly.
- Activity `if`/`else`/`endif`: the condition and merge diamonds sit between the branch centres, so the
  branch arrows meet the condition vertices and the return arrows meet the merge diamond. The `yes` and
  `no` labels are placed at the vertices and drawn after the shapes, instead of the `no` label being
  covered by the condition.

### Compatibility

- Additive only: `klimt`-level images can now report member rows (`IEntityImage::ports`,
  `svek::types::PortGeometry`), and the layout accepts explicit node/rank separation
  (`sdot::Spacing`, `sdot::build_image_spaced`). Existing callers and the top-level render API are
  unchanged.

## [0.2.0] - 2026-10-27

### Fixed

- Ordinary and non-breaking spaces now have a visible font advance, fixing collapsed words in C4
  descriptions and other wrapped labels. Text widths and diagram layouts can change as a result.
- Label sprites use the active font colour; the C4 person icon is white on its dark blue background.
- VS Code Markdown integration returns its renderer API from activation, so `plantuml`, `puml`, and
  `uml` fences render in the built-in Markdown preview. Bare statements are wrapped in a diagram block.
- The VS Code preview command is available in editor title and context menus, with rendering diagnostics
  available through `PlantUML: Show Log`.

### Compatibility

- The public `klimt::creole::atoms::Atom::Sprite` variant now carries the label colour as a second field.
  Downstream code constructing or matching this variant must be updated. The top-level render API is unchanged.
- Fidelity measurements after correcting space widths are recorded in `docs/PLAN.md`; references and
  acceptance rules have not been relaxed.

## [0.1.6] - 2026-10-27

No changes to the library or the CLI: this release republishes the 0.1.5 sources.

## [0.1.5] - 2026-10-27

No changes to the library or the CLI: this release republishes the 0.1.4 sources.

## [0.1.4] - 2026-10-27

No changes to the library or the CLI: this release republishes the 0.1.3 sources.

## [0.1.3] - 2026-10-27

No changes to the library or the CLI: this release republishes the 0.1.2 sources.

## [0.1.2] - 2026-10-27

EBNF rendering fixes; no API change.

- Terminal boxes in EBNF diagrams are drawn as outlines again instead of being filled with the ambient
  colour, and the entry marker is an open circle as in Java.
- Connector arcs in EBNF diagrams are stroked (`fill="none"`, stroke-width 1.5) instead of being drawn as
  filled shapes.
- EBNF boxes and rule names are sized from the line box (ascent + descent + line gap) that Java uses, so a
  grammar with several productions no longer drifts apart.
- The repository gained `fixtures/` with EBNF and C4 sources and PlantUML-generated references, and
  `scripts/fidelity.py` now scores them alongside the Vega corpus; the VS Code extension lives in
  `editors/vscode`.

## [0.1.1] - 2026-10-26

Documentation, CI and publishing only: the library and the CLI are unchanged from 0.1.0.

- Publishing to crates.io now runs from CI on a pushed `v*` tag, using crates.io trusted publishing
  (OIDC) instead of a stored API token, with a tag/version check, a main-branch check, the test suite and
  an already-published skip per crate.
- The playground gained samples for C4 and EBNF, the two supported diagram types that had none, and the
  supported-types label lists them.
- `docs/PERFORMANCE.md` now reports the current measurements (3.07 ms per diagram against 2.80 ms for
  Java PlantUML in one JVM) and the profile behind them; the plan dropped its historical progress log.

## [0.1.0] - first release

- `plantuml-core`: renders PlantUML sources to SVG with no I/O, for native and `wasm32-unknown-unknown`.
  Supported diagram types: sequence, class (with object and description diagrams), state, activity,
  packet, chart, gantt, timing, mindmap, WBS, salt, JSON/YAML/HCL, Chen EER, EBNF and the crash
  diagram; C4-PlantUML renders through the bundled stdlib. Includes the embedded font metrics, the
  bundled themes and the C4 stdlib sources.
- `plantuml-cli`: reads a diagram from a file or stdin and writes SVG to stdout.
- Fidelity against the Vega fixtures: 431 render, 391 of 391 scored fixtures meet the target, 327 are
  structurally identical; the accepted differences are listed in `docs/PLAN.md`.
- Rendering performance: 3.07 ms per diagram on the 472-file corpus against 2.77 ms for Java PlantUML
  in one JVM, see `docs/PERFORMANCE.md`.
- Not included in this release: `plantuml-wasm` (published separately once the npm package lands), the
  diagram types listed as out of scope in `docs/PLAN.md`, and embedded `{{ … }}` diagrams.
