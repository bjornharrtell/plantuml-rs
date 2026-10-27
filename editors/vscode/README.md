# PlantUML (Rust) for VS Code

Renders PlantUML locally, with no Java, no Graphviz process and no network: the extension bundles a wasm build
of [`plantuml-core`](../../crates/plantuml-core).

- **`.puml` preview**: `PlantUML: Open Preview` (`plantuml-rs.openPreview`) opens a side panel for the active
  document and updates as you type.
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

Before publishing, set `publisher` in `package.json` to the Marketplace publisher id and log in with
`vsce login <publisher>`. Publishing to Open VSX instead is `npx ovsx publish -p <token>` with a namespace.

The extension has no runtime dependencies: `markdown-it` is only a dev dependency for the tests.
