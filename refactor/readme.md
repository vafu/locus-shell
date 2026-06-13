# Locus Shell Review Notes

## Session Scope

Interactive review of the `locus-shell` framework workspace. The default deliverable is durable understanding, reviewer notes, open questions, and concrete refactor plans. Implementation changes are out of scope unless explicitly requested during the review.

## Global Architecture Constraints

- `shell/core` owns generic GTK4/Relm4/layer-shell framework primitives only.
- Consumer crates own widget roles such as bars, OSDs, notifications, launchers, and workspace switchers.
- `dev-widgets` is internal development code for testing framework ergonomics, not a user-facing implementation.
- Styling belongs in external CSS/SCSS files rather than hardcoded Rust widget properties.
- D-Bus work should run asynchronously outside the GTK UI thread.
- Locus graph state should be driven by `io.github.Locus.Graph.Resolve`, `SubscribeResolve`, and `ResolveChanged`; clients should not invent a separate reactive runtime.
- Schema-specific Locus markers, paths, relations, and extension traits belong in consuming crates, currently represented by `dev-widgets` development schema code.
- Provider crates should remain backend-neutral at the contract layer and avoid GTK, Relm4, or product-specific shell policy.

## Review Queue

All workspace units have first-pass notes.

## Completed Units

- `shell-core` (`shell/core`) - notes in `refactor/shell-core.md`.
- `providers` (`provider/core`) - notes in `refactor/providers.md`.
- `property-provider` (`provider/property`) - notes in `refactor/property-provider.md`.
- `locus-provider` (`provider/locus`) - notes in `refactor/locus-provider.md`.
- `dbus-provider` (`provider/dbus`) - notes in `refactor/dbus-provider.md`.
- `common-providers` (`provider/common`) - notes in `refactor/common-providers.md`.
- `shell-macros` (`shell/macros`) - notes in `refactor/shell-macros.md`.
- `dev-widgets` (`dev-widgets`) - notes in `refactor/dev-widgets.md`.

## Cross-Cutting Findings

- `shell-core`: stylesheet watching uses modification-time fingerprinting; revisit if development reload correctness matters more than lightweight polling.
- `providers`: core refactor is complete. The contract is now stream-native with direct Tokio `CancellationToken`, and downstream workspace crates have migrated from the removed callback API.
- `locus-provider`: descriptor fields are now private behind accessors; collection kind filtering can still become N+1 D-Bus work and remains a runtime hardening item.
- `dbus-provider`: initial property reads happen before property-change subscription, so strict no-gap watching needs a closer look.
- `common-providers`: boundary is clean; future ergonomics question is whether raw integer-coded properties should gain typed enums.
- `shell-macros`: generated-code confidence needs compile-expanded tests, especially around typed model view binding validation and wrapped component inputs.
- `dev-widgets`: boundary is clean; generated schema helpers and component-backed rows now match the current roadmap direction.

## User-Wide Guidance

- Keep factual behavior, user notes, findings, refactor items, and open questions separate.
- Mark inferences explicitly.
- Cross-reference `PLAN.md` before accepting architecture changes that shift responsibility boundaries.
- Do not update `PLAN.md` unless the user agrees the roadmap has changed.
- During explicit review, proceed file by file: explain the next file briefly, separate facts from opinions, pause for user questions, then capture user notes before moving on.
- Collect action points in the relevant `refactor/*.md` files during review. Do not execute them immediately; execute accepted actions at the end of the review pass.
- Macro/source integration should be backend-agnostic: DBus and Locus source expressions should both compile through `providers::Provider<T>`, new macro usage should prefer `#[source(...)]`, and generated messages should carry `Result<T, E>` per field.
- Property-backed providers are a distinct provider family. The `provider/property` crate now holds shared property descriptors and property-binding traits; DBus and Locus should implement those traits while macros continue to depend only on `providers::Provider<T>`.

## Deferred Action Points

None.

## Open Questions

- Should row selection be lifted to parent-owned shared state if repeated selected-window subscriptions become measurable overhead?
- Should property helper wrappers such as `with_default` live in `property-provider`, or remain generic provider/stream adapters until repeated use proves a property-specific need?
