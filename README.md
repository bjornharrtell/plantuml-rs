# plantuml-rs

Rust/wasm port of [PlantUML](https://github.com/plantuml/plantuml): renders diagrams to SVG in the
browser with no backend. See [docs/PLAN.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PLAN.md) for the porting decisions, the fidelity
target and the current per-type status.

```sh
cargo install plantuml-cli          # or: cargo run --release -p plantuml-cli -- diagram.puml
plantuml-cli diagram.puml > diagram.svg
```

The library is `plantuml-core` (`cargo add plantuml-core`). The API is unstable until 1.0; see
[CHANGELOG.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/CHANGELOG.md) for what a release contains.

## Status

391 of 391 scored Vega fixtures meet the fidelity target (text overlap >= 0.95, relative element
placement >= 0.90), 327 are structurally identical, and every one of the 431 reference fixtures
renders. 40 fixtures are documented accepted differences (activity tile geometry, embedded
diagrams, handwritten mode and residual layout deltas); none of them is a missing feature or a
crash. Supported today: sequence, class (with object and description diagrams), state, activity,
packet, chart, gantt, timing, mindmap, WBS, salt, JSON/YAML/HCL, Chen EER, EBNF and the crash
diagram. C4-PlantUML renders through the bundled stdlib. Accepted differences and open items are
listed in the plan.

## Performance

On the 472-file Vega corpus the library renders 5.7 ms per diagram against 2.8 ms for Java PlantUML in
one JVM, with about 76 ms of one-time initialization per process. Method, caveats and reproduction are
in [docs/PERFORMANCE.md](https://github.com/bjornharrtell/plantuml-rs/blob/main/docs/PERFORMANCE.md).

## License

MIT. Ported code derives from PlantUML under its MIT option; `graphviz-pure` is EPL-2.0.
