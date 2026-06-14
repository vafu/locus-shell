# Window Lifecycle Handles

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)
- [Bar](../migration/widgets/bar.md)

## Gap

Consumer monitor managers need to own, close, and replace Relm4/layer-shell
windows cleanly.

## Direction

Use Relm4 controller handles directly at first. Add `shell-core` helper types
only if lifecycle code repeats across binaries.

