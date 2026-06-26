# Consumer Request Service

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

`rsynapse-shell` needs an app-owned request bus for commands such as
`scheme-toggle`, hints state, and approval open.

## Direction

Implement in the consumer crate or runtime binary. Do not move command names or
product policy into `shell-core`.

## Status

Initial implementation exists in `rsynapse-shell`: the binary has a
Unix-socket `request` client/server for `scheme-toggle` and `hints` commands.
Future approval/search commands can reuse the same consumer-local request
module when those UI surfaces are ported.
