# Carousel Widget

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Approval cards need paged navigation and indicator dots over a dynamic
collection.

## Direction

Implement in `rsynapse-shell` first using Relm4 child components. Promote a
generic helper only if another overlay needs the same interaction.

