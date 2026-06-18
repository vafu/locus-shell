# Observable Source Composition

## Needed By

- [Bar](../migration/widgets/bar.md)
- [Agent approvals](../migration/widgets/agent-approvals.md)

## Gap

Migrated widgets need a concise way to derive UI DTOs from multiple Locus,
D-Bus, timer, file, and service sources without custom source structs or
manual watcher loops.

## Direction

Implement the Observable source API from `../../SOURCE_API.md`:

- model fields keep `#[source(...)]` and store plain values;
- user-authored derived sources use `#[shell_macros::observable]`;
- derived source dependencies use `#[observe(...)]`;
- stable runtime/config services use `#[inject]`;
- generated source wiring owns subscription lifecycle, errors, sharing, and
  Relm4 message forwarding.

Keep backend graph/property subscriptions generated or backend-owned. Widget
code should compose observables and return summarized typed models.
