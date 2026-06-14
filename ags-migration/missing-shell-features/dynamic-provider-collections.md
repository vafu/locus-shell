# Dynamic Provider Collections

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Collection providers need to hydrate changing item lists where each item has
its own provider dependencies.

## Direction

Implement custom stream functions for list hydration first. Add reusable
helpers only after workspace/window/build/session use cases converge.

