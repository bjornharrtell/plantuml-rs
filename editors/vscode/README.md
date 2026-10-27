# PlantUML (Rust) for VS Code

Renders PlantUML locally, with no Java, no Graphviz process and no network: the extension bundles a wasm build
of [`plantuml-core`](../../crates/plantuml-core).

- **`.puml` preview**: use **PlantUML: Open Preview** from the Command Palette, editor title menu, or
  editor context menu. It opens a side panel for the active document and updates as you type.
- **Syntax highlighting** for `.puml`, `.plantuml`, `.iuml` and `.pu`, with comment toggling and bracket
  matching (`syntaxes/plantuml.tmLanguage.json`, `language-configuration.json`).
- **PlantUML: Export SVG** writes the current diagram to a `.svg` next to the document.
- **Markdown**: ```puml / ```plantuml / ```uml fenced blocks render inside VS Code's built-in Markdown preview
  (contributed through `markdown.markdownItPlugins`).
- A diagram the port cannot render is shown as a red message box with the error, instead of failing silently.

## Build and test

The wasm package is generated, not committed:

```sh
../..//scripts/build-vscode-ext.sh        # cargo build + wasm-bindgen --target nodejs -> wasm/
npm install && npm test                   # renders diagrams and the markdown fences in Node
```

Open the folder in VS Code and press `F5` to launch an Extension Development Host.

## Package and publish

```sh
../../scripts/build-vscode-ext.sh -p      # wasm + npm install + vsce package -> plantuml-rs.vsix
npx @vscode/vsce publish                  # needs the publisher id below and a PAT
```

Publishing is manual: log in with `vsce login bjornharrtell`, then `npx @vscode/vsce publish --packagePath
plantuml-rs.vsix`. For Open VSX use `npx ovsx publish plantuml-rs.vsix -p <token>`. The `version` in
`package.json` follows the crate version: `chore(release)` bumps all of them together.

The extension has no runtime dependencies: `markdown-it` is only a dev dependency for the tests.
