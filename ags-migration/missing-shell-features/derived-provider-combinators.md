# Derived Provider Combinators

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)
- [Bar](../migration/widgets/bar.md)

## Gap

Several widgets need derived values from multiple typed sources. The user-facing
provider code should express the data join, not the runtime mechanics.

## Direction

Use stream-level helpers in `providers`, starting with existing
`combine_latest2_stream` / `combine_latest2`. Also evaluate `rxrust` as a
consumer-side composition layer if it removes enough switch/combine boilerplate
without leaking Rx-specific concepts into generated schema APIs. Add only
concrete helpers needed by migrated widgets.
