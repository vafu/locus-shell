# Layer Window Monitor Rebinding

## Needed By

- [OSD](../migration/widgets/osd.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Singleton overlays need to move to the active monitor without recreating all UI
state.

## Direction

Add consumer helpers around GTK monitor references and layer-shell monitor
configuration. This may share implementation with per-monitor lifecycle.
