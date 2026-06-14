# CLI Request Bridge

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)
- [App runtime](../migration/widgets/app-runtime.md)

## Gap

AGS exposes `ags request`; the Rust shell needs a typed command path for
opening overlays and changing runtime state.

## Direction

Use a small `rsynapsectl` CLI over a session D-Bus request interface owned by
`rsynapse-shell`.
