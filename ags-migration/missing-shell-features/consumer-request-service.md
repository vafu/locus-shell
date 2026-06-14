# Consumer Request Service

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

`rsynapse-shell` needs an app-owned request bus for commands such as
`scheme-toggle`, hints state, and approval open.

## Direction

Implement in the consumer crate or runtime binary. Do not move command names or
product policy into `shell-core`.
