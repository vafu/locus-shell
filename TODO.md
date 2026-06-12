# TODO

Track concrete follow-up work that is not yet ready to become a committed roadmap step.

## Watch

- Keep consumer-facing Locus APIs semantic. Raw graph direction such as `sources`,
  `targets`, `SubscribeSources`, and `SubscribeTargets` should stay private or
  explicitly low-level once generated path/collection APIs exist.
- `Path<Workspace>::windows()` currently filters node `kind == "window"` because
  the current Locus schema exposes `workspace` as an overloaded relation. Replace
  this hand-written kind filter with generated typed relation/collection metadata
  when Locus codegen owns relation descriptors.
- Dynamic row widgets need explicit composition support. The macro now supports
  wrapping generated source messages in a component input enum, but the next
  window-list UI slice should decide whether to use Relm4 factories directly or
  add a small shell macro helper for per-row provider subscriptions.

# Optional

- Consider backend-specific subscription caches for `locus-provider` and
  `dbus-provider` if repeated widget composition starts creating duplicate live
  watches. The cache should be a shared subscription registry, not only a value
  cache: one D-Bus/Locus watch per canonical object/node/path key, local fan-out
  to subscribers, latest-value replay, and cleanup when the last subscriber
  drops. Keep this out of `shell-core` and macros; backend runtimes own their
  cache keys and invalidation semantics.
