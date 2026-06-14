# Command Action Provider

## Needed By

- [Bar](../migration/widgets/bar.md)
- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Some UI actions invoke external commands such as notification toggles, DND
scripts, suspend helpers, and stats scripts.

## Direction

Keep product-specific commands in `rsynapse-shell`. Add a small typed side
effect runner or process-output provider when repeated command handling needs
shared cancellation/error behavior.

