# Dynamic CSS Reload

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

AGS regenerates and reloads CSS at runtime for dynamic theme values.

## Direction

Use `shell-core` stylesheet registration and file watching where possible.
Keep dynamic CSS generation in `rsynapse-shell`.

