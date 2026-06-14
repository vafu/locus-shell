# GtkSourceView Integration

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Approval details need readable diff/source rendering with syntax highlighting.

## Direction

Add `sourceview5`/GtkSourceView integration in `rsynapse-shell` first. Keep it
out of `shell-core` unless other consumers need a generic source viewer.

