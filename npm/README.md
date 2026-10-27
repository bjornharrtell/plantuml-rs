# plantuml-rs

PlantUML rendering in WebAssembly. `render(source)` returns an SVG string, locally: no Java, no
Graphviz process, no network. Output is deterministic for the same input and version.

Port of [PlantUML](https://github.com/plantuml/plantuml) to Rust; the crate is
[`plantuml-core`](https://crates.io/crates/plantuml-core).

## Node

```js
const { render } = require('plantuml-rs');

const svg = render(`@startuml
Alice -> Bob: hello
@enduml`);
```

ESM works too: `import { render } from 'plantuml-rs';`

## Browser / bundlers

The default export for browsers is the ESM build, which needs one `init()` call to fetch its `.wasm`:

```js
import init, { render } from 'plantuml-rs';

await init();
document.querySelector('#preview').innerHTML = render(`@startuml
class Foo
@enduml`);
```

`init({ module_or_path: wasmBytes })` also accepts the wasm bytes when you serve or bundle them yourself:
`import wasmUrl from 'plantuml-rs/dist/web/plantuml_wasm_bg.wasm?url'`.

In Node without a bundler you can import the web build directly and pass the bytes:

```js
import { readFile } from 'node:fs/promises';
import init, { render } from 'plantuml-rs/web';

await init({
  module_or_path: await readFile(
    new URL('plantuml-rs/dist/web/plantuml_wasm_bg.wasm', import.meta.url),
  ),
});
```

## API

- `render(source: string): string` — renders the first `@start…@end` block to SVG. Throws an `Error`
  with the parser message (including the line where known) when the diagram cannot be rendered.
- `init_logging()` — routes Rust `log` output to the console; optional.

Supported diagrams: sequence, class, object, state, activity (beta), component/description, use case,
C4-PlantUML, Chen EER, packet, chart, gantt, timing, mindmap, WBS, salt, JSON/YAML/HCL, EBNF and the
crash diagram. See the repository for the fidelity report and the accepted differences.

## Notes

- The `.wasm` is ~6 MB uncompressed (it embeds fonts and the C4-PlantUML library); serve it gzip/brotli.
- No I/O: `!include` resolves the bundled standard library only, and remote includes are not fetched.
- Licence: MIT, like the port. It depends on `graphviz-pure` (EPL-2.0) and bundles Liberation fonts.
