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

All initial review units have first-pass notes.

## Completed Units

- `shell-core` (`shell/core`) - notes in `refactor/shell-core.md`.
- `providers` (`provider/core`) - notes in `refactor/providers.md`.
- `locus-provider` (`provider/locus`) - notes in `refactor/locus-provider.md`.
- `dbus-provider` (`provider/dbus`) - notes in `refactor/dbus-provider.md`.
- `common-providers` (`provider/common`) - notes in `refactor/common-providers.md`.
- `shell-macros` (`shell/macros`) - notes in `refactor/shell-macros.md`.
- `dev-widgets` (`dev-widgets`) - notes in `refactor/dev-widgets.md`.

## Cross-Cutting Findings

- `shell-core`: stylesheet watching uses modification-time fingerprinting; revisit if development reload correctness matters more than lightweight polling.
- `providers`: core refactor is complete. The contract is now stream-native with direct Tokio `CancellationToken`; downstream crates still need migration from the removed callback API.
- `locus-provider`: descriptor fields are still raw/public and collection kind filtering can become N+1 D-Bus work; both align with roadmap hardening items.
- `dbus-provider`: initial property reads happen before property-change subscription, so strict no-gap watching needs a closer look.
- `common-providers`: boundary is clean; future ergonomics question is whether raw integer-coded properties should gain typed enums.
- `shell-macros`: generated-code confidence needs compile-expanded tests, especially around typed model view binding validation and wrapped component inputs.
- `dev-widgets`: boundary is clean; manual schema extension traits and child reconciliation match the roadmap's next evaluation points.

## User-Wide Guidance

- Keep factual behavior, user notes, findings, refactor items, and open questions separate.
- Mark inferences explicitly.
- Cross-reference `PLAN.md` before accepting architecture changes that shift responsibility boundaries.
- Do not update `PLAN.md` unless the user agrees the roadmap has changed.
- During explicit review, proceed file by file: explain the next file briefly, separate facts from opinions, pause for user questions, then capture user notes before moving on.
- Collect action points in the relevant `refactor/*.md` files during review. Do not execute them immediately; execute accepted actions at the end of the review pass.

## Deferred Action Points

- Tighten `rust-guide` wording so public API traits, structs, type aliases, and re-exports belong in `mod.rs` or `lib.rs`, while implementation files stay focused on implementation details.

## Open Questions

- Which review unit should be read first: the roadmap order above, or a specific crate the user wants to inspect now?
- Should review notes include line-level findings only after a full crate pass, or should likely findings be captured immediately as they appear?
