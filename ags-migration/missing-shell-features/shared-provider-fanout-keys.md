# Shared Provider Fanout Keys

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Many row components will subscribe to the same expensive upstream source unless
sharing is applied from stable source keys.

## Direction

Keep `SharedProvider` as the runtime primitive. Add descriptor-keyed sharing in
backend/generated providers where keys are stable.

