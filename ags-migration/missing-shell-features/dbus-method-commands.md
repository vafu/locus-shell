# D-Bus Method Commands

## Needed By

- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Current provider crates focus on properties; widgets also need typed method
calls for actions such as responding to approvals.

## Direction

Add consumer/provider clients for typed method calls. Consider shared
`dbus-provider` helpers for request/response calls once common patterns appear.
