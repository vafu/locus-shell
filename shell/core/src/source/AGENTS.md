# shell_core::source Instructions

This module is the Observable bridge for locusfs and other source backends.

- Public APIs in `shell_core::source` must return `Observable<...>`, expose
  types needed to consume those observables, or provide minimal LocusPath
  construction/conversion helpers used to create observables.
- `mod.rs` is the public contract. Private implementation modules may use plain
  `pub` for items shared by sibling files; do not re-export those items from
  `mod.rs` unless they are part of the observable source API.
- Do not add public one-shot read helpers, imperative clients, imperative
  snapshot functions, async-loop builders, consumer-facing `Future`/`Stream`
  factories, or child-snapshot convenience APIs. Dynamic child view models
  should be composed from `children()` plus per-child property/relation
  observables.
- Handwritten async/watch loops and direct `locusfs-watch` client usage belong
  only in small private implementation files that create observables.
- Consumer crates, including `rsynapse-shell`, should compose these observables
  with Rx operators and must not depend on the `locusfs-watch` client feature.
- If a new backend capability is needed, expose it as a reusable observable
  primitive from this module rather than leaking backend reads to consumers.
