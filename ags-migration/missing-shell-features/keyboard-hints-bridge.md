# Keyboard Hints Bridge

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Triggerhappy currently talks to AGS requests to toggle hints mode.

## Direction

Replace the AGS request target with `rsynapsectl` and expose hints state as
consumer runtime state or a provider.

