# Derived Locus Collections

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)
- [Bar](../migration/widgets/bar.md)

## Gap

Widgets need semantic collections such as selected-workspace windows,
window-agent-session targets, and hydrated workspace rows.

## Direction

Prefer generated schema helpers and consumer provider functions that return
typed DTO lists. Use provider stream helpers for joins, but keep semantic graph
knowledge out of widget views.

