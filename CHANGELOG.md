# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the API is unstable until 1.0.

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
