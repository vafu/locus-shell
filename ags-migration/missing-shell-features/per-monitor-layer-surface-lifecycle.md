# Per-Monitor Layer Surface Lifecycle

## Needed By

- [Bar](../migration/widgets/bar.md)
- [App runtime](../migration/widgets/app-runtime.md)

## Gap

The bar needs one layer-shell window per monitor with add/remove reconciliation.

## Direction

Implement consumer-owned monitor lifecycle manager first. Extract generic
handles into `shell-core` only if needed beyond `rsynapse-shell`.

