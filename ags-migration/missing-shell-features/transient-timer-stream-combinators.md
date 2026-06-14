# Transient Timer Stream Combinators

## Needed By

- [OSD](../migration/widgets/osd.md)

## Gap

OSD level events need restartable delayed hide behavior.

## Direction

Start with an OSD-local command task or stream. Add a reusable debounce/timer
helper only if other overlays need the same semantics.
