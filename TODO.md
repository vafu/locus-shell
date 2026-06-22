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
- Move PipeWire volume changes away from debounce-style burst collapse in
  `../locusfs/plugins/pipewire`. The current plugin waits for a pactl
  subscription burst to settle, then publishes one full snapshot, so rapid
  volume changes lose intermediate states before `rsynapse-shell` can observe
  them.
- Add a live locusfs MPRIS projection. The Rust bar now consumes the intended
  `/mpris/player/*` observable shape, but the generic locusfs D-Bus projection
  only snapshots service objects on owner changes and is not enough for MPRIS
  metadata/playback property updates.
- Extend the locusfs PipeWire projection with route grouping metadata such as
  device id and session priority, or expose a normalized route list. The Rust
  audio selector currently lists sinks directly because the plugin does not yet
  expose the data AGS used to deduplicate routes by physical device.

# Optional

- Add backend subscription sharing for `locus-provider` and `dbus-provider`
  using stable backend keys. This
  should be a shared subscription registry, not only a value cache: one
  D-Bus/Locus watch per canonical object/node/path key, local fan-out to
  subscribers, latest-value replay, and cleanup when the last subscriber drops.
  Keep this out of widget code; backend runtimes own their cache keys and
  invalidation semantics.
