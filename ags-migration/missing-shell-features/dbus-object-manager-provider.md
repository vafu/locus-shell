# D-Bus ObjectManager Provider

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)
- [Bar](../migration/widgets/bar.md)

## Gap

Agent sessions are dynamic D-Bus objects discovered through ObjectManager.

## Direction

Add an ObjectManager collection provider that emits typed maps/lists from
initial `GetManagedObjects`, `InterfacesAdded`, `InterfacesRemoved`, and
`PropertiesChanged`.

