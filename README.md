# plantuml-rs

Rust/wasm port of [PlantUML](https://github.com/plantuml/plantuml): renders diagrams to SVG in the
browser with no backend. See [docs/PLAN.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PLAN.md) for the porting decisions, the fidelity
target and the current per-type status.

```sh
cargo install plantuml-cli          # or: cargo run --release -p plantuml-cli -- diagram.puml
plantuml-cli diagram.puml > diagram.svg
```

The library is `plantuml-core` (`cargo add plantuml-core`). The API is unstable until 1.0; see
[CHANGELOG.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/CHANGELOG.md) for what a release contains
and [docs/RELEASING.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/RELEASING.md) for how releases
are cut (publishing to crates.io runs on pushed tags).

## Status

378 of 382 scored fixtures meet the fidelity target (text overlap >= 0.95, relative element
placement >= 0.90), 185 are structurally identical, and every one of the 441 scored fixtures renders.
59 fixtures are documented accepted differences (activity tile geometry, embedded diagrams,
handwritten mode, residual layout deltas and the 24 fixtures whose reference was rendered with
zero-width spaces); none of them is a missing feature or a crash. Supported today: sequence,
class (with object and description diagrams), state, activity, packet, chart, gantt, timing,
mindmap, WBS, salt, JSON/YAML/HCL, Chen EER, EBNF and the crash diagram. C4-PlantUML renders
through the bundled stdlib. Four C4/EBNF fixtures still miss the layout target; accepted
differences and open items are listed in the plan.

## Performance

On the 472-file Vega corpus the library renders 3.1 ms per diagram against 2.8 ms for Java PlantUML in
one JVM — about 10% behind — with roughly 76 ms of one-time initialization per process. Method,
caveats and the profile behind those numbers are in
[docs/PERFORMANCE.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PERFORMANCE.md).

## License

MIT. Ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0.
