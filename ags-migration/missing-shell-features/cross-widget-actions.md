# Cross Widget Actions

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Runtime logic may need to open an overlay based on provider state, for example
auto-opening approvals for the selected workspace.

## Direction

Use typed requests/messages between `rsynapse-shell` modules or binaries. Keep
the policy in the consumer layer.

