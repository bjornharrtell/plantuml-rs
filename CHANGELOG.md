# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the API is unstable until 1.0.

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
