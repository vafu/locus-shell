# Keyboard Shortcuts

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Overlays need ergonomic key handling for navigation, activation, dismissal, and
option selection.

## Direction

Start with explicit GTK event controllers in consumer components. Consider
`shell-core` helpers only if repeated patterns become obvious.
