# Provider Stream Combinators

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Migrated widgets need narrow stream helpers for derived DTOs. Consumer providers
should look like typed data composition functions and hide watcher loops,
subscription wiring, switch/restart behavior, and fanout boilerplate.

## Direction

Use current `combine_latest2` for simple joins while evaluating whether
`rxrust` gives a cleaner consumer composition layer. Keep generated Locus/DBus
providers as the source of graph/property subscriptions. Add `combine_latest3`,
debounce, distinct, filter-map, or switch-map only after widget code requires
them or after an RxRust spike proves the dependency reduces local runtime code.
