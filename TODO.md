# TODO

Track concrete follow-up work that is not yet ready to become a committed roadmap step.

## Watch

- Keep consumer-facing Locus APIs semantic. Raw graph direction such as `sources`,
  `targets`, `SubscribeSources`, and `SubscribeTargets` should stay private or
  explicitly low-level once generated path/collection APIs exist.
- Generated schema collection metadata now owns `Path<Workspace>::windows()`.
  Keep watching whether overloaded Locus relations need richer schema syntax as
  more collection helpers appear.
- Dynamic row widgets now use small child components with local provider
  bindings declared by macro-level `#[bind_list(..., row = Component)]` and
  hosted by the GTK list container. Keep watching whether the component-list
  path needs explicit key/sort hooks beyond value equality.
- Generated schema collection providers now expose typed node references instead
  of raw node-id strings. Future list cleanup should add GTK-native, Adwaita,
  and custom add/remove adapters without making those backends mandatory
  `shell-core` dependencies.

# Optional

- Add backend subscription sharing for `locus-provider` and `dbus-provider`
  using provider-core shared-latest mechanics and stable backend keys. This
  should be a shared subscription registry, not only a value cache: one
  D-Bus/Locus watch per canonical object/node/path key, local fan-out to
  subscribers, latest-value replay, and cleanup when the last subscriber drops.
  Keep this out of widget code; backend runtimes own their cache keys and
  invalidation semantics.
