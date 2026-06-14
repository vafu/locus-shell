# Typed Locus Collection Hydration

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Widgets need graph lists converted into typed row DTOs with properties and
relations already resolved.

The first `rsynapse-shell` project-label provider currently snapshots
workspace/project fields whenever selected output, selected workspace, or the
workspace list changes. It does not yet re-emit when an existing workspace's
project display properties change.

## Direction

Generate or write schema helper providers such as `workspace.window_rows()` and
`selected_workspace.agent_sessions()`. Keep hydration logic out of views.
