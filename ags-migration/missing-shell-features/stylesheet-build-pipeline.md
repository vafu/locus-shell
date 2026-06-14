# Stylesheet Build Pipeline

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)
- [All migrated widgets](../migration/widgets/README.md)

## Gap

AGS dynamically compiles SCSS and bridges GTK color definitions into SCSS
variables.

## Direction

Use `shell-core` stylesheet loading/watching, but keep the SCSS preprocessing
and style ownership in `rsynapse-shell`.

