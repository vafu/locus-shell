# Shared Runtime State

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Hints mode, theme state, and command-visible overlay state may be consumed by
multiple widgets or binaries.

## Direction

Prefer explicit request/provider contracts over global mutable state. If a
coordinator binary exists, expose typed D-Bus state.

