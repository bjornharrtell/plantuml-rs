# plantuml-rs

Rust/wasm port of [PlantUML](https://github.com/plantuml/plantuml): renders diagrams to SVG in the
browser with no backend. See [docs/PLAN.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PLAN.md) for the porting decisions, the fidelity
target and the current per-type status.

```sh
cargo install plantuml-cli          # or: cargo run --release -p plantuml-cli -- diagram.puml
plantuml-cli diagram.puml > diagram.svg
plantuml-cli -o build a.puml b.puml # one SVG per input
plantuml-cli --check *.puml         # validate in CI, non-zero exit on failure
plantuml-cli --json diagram.puml    # type, canvas size and SVG as JSON
```

Pre-built binaries for Linux, macOS (x64/arm64) and Windows are attached to every
[GitHub release](https://github.com/bjornharrtell/plantuml-rs/releases), with checksums.

The library is `plantuml-core` (`cargo add plantuml-core`); the wasm build is on npm as
[`plantuml-rs`](https://www.npmjs.com/package/plantuml-rs) (see [JavaScript](#javascript)). The API is
unstable until 1.0; see
[CHANGELOG.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/CHANGELOG.md) for what a release contains
and [docs/RELEASING.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/RELEASING.md) for how releases
are cut (publishing to crates.io runs on pushed tags).

## Status

377 of 381 scored fixtures meet the fidelity target (text overlap >= 0.95, relative element
placement >= 0.90), and every one of the 442 scored fixtures renders. 61 fixtures are documented
accepted differences (activity tile geometry, embedded diagrams, handwritten mode, residual layout
deltas, the 24 fixtures whose reference was rendered with zero-width spaces and the 5 laid out with
the shorter line box); none of them is a missing feature or a crash. Against a live Java render the
port scores text 0.91 and layout 0.71 over the 391 fixtures Java can render. Supported today:
sequence, class (with object and description diagrams), state, activity, packet, chart, gantt, timing,
mindmap, WBS, salt, JSON/YAML/HCL, Chen EER, EBNF and the crash diagram. C4-PlantUML renders
through the bundled stdlib. Four C4/EBNF fixtures still miss the layout target; accepted
differences and open items are listed in the plan.

## JavaScript

```js
const { render } = require('plantuml-rs');       // Node; ESM import works too
const svg = render('@startuml\nAlice -> Bob: hello\n@enduml');
```

In the browser the ESM build needs one `await init()` before `render`; see the
[package README](https://github.com/bjornharrtell/plantuml-rs/blob/main/npm/README.md). It is the same
`plantuml-core`, compiled to wasm: no Java, no Graphviz, no network.

## Editors

The VS Code extension [PlantUML (plantuml-rs)](https://marketplace.visualstudio.com/items?itemName=bjornharrtell.plantuml-rs)
([Open VSX](https://open-vsx.org/extension/bjornharrtell/plantuml-rs)) previews `.puml` files and renders
`puml` fences in the built-in Markdown preview locally through the wasm build: syntax highlighting,
**PlantUML: Export SVG**, no Java, no Graphviz process and no network. Sources are in
[`editors/vscode`](editors/vscode).

## Performance

On the 472-file Vega corpus the library renders 1.8 ms per diagram against 2.5 ms for Java PlantUML in
one JVM — about 27% ahead — with roughly 76 ms of one-time initialization per process. Method,
caveats and the profile behind those numbers are in
[docs/PERFORMANCE.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PERFORMANCE.md).

## License

MIT. Ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0.
