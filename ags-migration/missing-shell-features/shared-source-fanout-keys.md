# Shared Source Fanout Keys

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Many row components will subscribe to the same expensive upstream source unless
sharing is applied from stable source keys.

## Direction

Add descriptor-keyed shared latest construction in backend/generated
ObservableSource code where keys are stable. Widget authors should not manage
local caches or manual sharing handles.
