# Reference fixtures for types the Vega corpus does not cover

`src/test/resources/vega` covers sixteen diagram types. EBNF and the C4 model set have no fixtures there, so
nothing measured them; these directories are their reference. The `*.svg` files are PlantUML's own output and the
`*.puml` files are the sources, so the same harness and metrics score them as the Vega corpus.

## Regenerating

With a Java runtime and the PlantUML jar built from the pinned checkout (`1.2026.10beta1`). The jar measures text
with whatever `sans-serif` the machine maps to (DejaVu Sans, Noto Sans, ...), which differs from the Liberation
Sans metrics the port models, so the render pins the face with a `defaultFontName` line after `@start...`:

```sh
cd fixtures
for f in */*.puml; do
  awk 'NR==FNR{if(!n&&/^@start/)n=FNR;next} {print} FNR==n{print "skinparam defaultFontName \"Liberation Sans\""}' "$f" "$f" |
    (cd "$(dirname "$f")" && java -jar /path/to/plantuml.jar -tsvg -pipe) > "${f%.puml}.svg"
done
```

Liberation Sans has to be installed (`fonts-liberation`). The C4 sources use `!include <C4/...>`, which the jar resolves from its bundled standard library.
