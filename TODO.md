# TODO

Track concrete follow-up work that is not yet ready to become a committed roadmap step.

## Watch

- Keep consumer-facing Locus APIs semantic. Raw graph direction such as `sources`,
  `targets`, `SubscribeSources`, and `SubscribeTargets` should stay private or
  explicitly low-level behind source functions.
- Keep watching whether overloaded Locus relations need richer source-function
  helpers as more collection helpers appear.
- Dynamic row widgets now use small child components with local provider
  bindings declared by macro-level `#[bind_list(..., row = Component)]` and
  hosted by the GTK list container. Keep watching whether the component-list
  path needs explicit key/sort hooks beyond value equality.
- Locus collection sources now expose raw node path strings. Future list cleanup
  should add GTK-native, Adwaita, and custom add/remove adapters without making
  those backends mandatory `shell-core` dependencies.
- Observable source API design is now tracked in `SOURCE_API.md`. Avoid adding
  new custom source composition APIs unless they are Observable migration work.

# Optional

- Add backend subscription sharing for `locus-provider` and `dbus-provider`
  using stable backend keys. This
  should be a shared subscription registry, not only a value cache: one
  D-Bus/Locus watch per canonical object/node/path key, local fan-out to
  subscribers, latest-value replay, and cleanup when the last subscriber drops.
  Keep this out of widget code; backend runtimes own their cache keys and
  invalidation semantics.
