# Reference fixtures for types the Vega corpus does not cover

`src/test/resources/vega` covers sixteen diagram types. EBNF and the C4 model set have no fixtures there, so
nothing measured them; these directories are their oracle. The `*.svg` files are PlantUML's own output and the
`*.puml` files are the sources, so the same harness and metrics score them as the Vega corpus.

## Regenerating

With a Java runtime and a PlantUML jar (these were generated with PlantUML 1.2026.9, the closest release to the
`1.2026.10beta1` base of the port):

```sh
cd fixtures
for f in */*.puml; do java -jar plantuml.jar -tsvg "$f"; done
```

The C4 sources use `!include <C4/...>`, which the jar resolves from its bundled standard library.
