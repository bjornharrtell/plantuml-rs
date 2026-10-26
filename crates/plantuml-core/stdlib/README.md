# Bundled standard library

Vendored sources for `!include <lib/name>`. `tim::stdlib` maps a reference to the files here.

## c4

[C4-PlantUML](https://github.com/plantuml-stdlib/C4-PlantUML), MIT (see `c4/LICENSE`), 3,361 lines across seven
`.puml` files. Vendor the files verbatim except for one change: the upstream guard

```
!if %variable_exists("RELATIVE_INCLUDE")
  !include ./C4_Context.puml
!else
  !include https://raw.githubusercontent.com/plantuml-stdlib/C4-PlantUML/master/C4_Context.puml
!endif
```

is rewritten to `!include <c4/C4_Context.puml>`, so the bundle includes itself instead of fetching from the network.
