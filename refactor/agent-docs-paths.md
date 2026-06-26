# Docs / Paths Review Plan

Review timestamp: 2026-06-26 01:02 PDT

Scope: docs and plans around observable sharing and locusfs path layout in
`SOURCE_API.md`, `PLAN.md`, `docs/audits/*.md`,
`ags-migration/missing-shell-features/shared-source-fanout-keys.md`, and
`rsynapse-shell/config/locusfs/config.toml`. I also checked current
`shell_core::source` implementation, affected `rsynapse-shell` D-Bus path
call sites, and adjacent `../locusfs/plugins/dbus` tests to verify the latest
D-Bus filesystem layout. No source code was edited.

Note: `docs/plans/config` does not exist in this checkout. Treat references to
that area as documentation drift unless a new directory is intentionally added.

## Executive Summary

The high-level source API decisions are already documented and mostly match the
current implementation:

- The public source model is Observable-first. `ObservableSource<T>` and the old
  provider runtime are not target APIs.
- Model fields hold plain values. Macro/generated code owns subscriptions,
  dirty tracking, and cancellation guards.
- Primitive backend sources should be shared and replay latest values by stable
  descriptor key so widget authors do not add `OnceLock` caches or manual
  `.shared()` calls.
- Blocking locusfs read/watch work must stay off the GTK thread.
- Source composition should use Rx operators, not local watcher loops or timing
  hacks.

The docs now lag the code in two important ways:

1. `docs/audits/shell-watch-paths.md` and
   `ags-migration/missing-shell-features/shared-source-fanout-keys.md` still
   describe descriptor-keyed sharing and relation path normalization as missing.
   Current `shell/core/src/source` has `shared_source`, `share_replay_latest`,
   relation target normalization, and tests for important path cases.
2. The new locusfs D-Bus layout is recorded in `AGENTS.md`, and confirmed by
   `../locusfs/plugins/dbus/src/state/test.rs`, but many shell consumers and
   migration notes still use legacy `object`, `@properties`, `@methods`, and
   `dbus-service` paths.

The next docs pass should convert stale audit items into status-tracked
decisions, then document the latest D-Bus path rules centrally before code is
migrated.

## Decisions Already Documented

### Observable API

`SOURCE_API.md` documents the target user-facing contract:

- Source expressions return `shell_core::source::Observable<T>`.
- Field-level `#[source(...)]` remains the model binding syntax.
- Derived source functions should use `#[shell_macros::observable]`,
  `#[observe(...)]`, and `#[inject]`.
- Reactive graph values are Observable dependencies, not DI services.
- Public composition should use RxRust operators such as `map`,
  `filter_map`, `combine_latest`, `switch_map`, and
  `distinct_until_changed`.

`PLAN.md` reinforces the same boundary:

- Do not reintroduce `Provider`, `ObservableSource`, custom source runtimes, or
  D-Bus graph compatibility layers.
- Keep direct `locusfs-watch` usage inside `shell_core::source`.
- Consumer crates compose `LocusPath` and shell-core Observable primitives.
- Reuse descriptor-keyed sharing where expected.

### Sharing Semantics

The intended sharing semantics are already stated in `SOURCE_API.md`:

- Backend sources are shared by descriptor key.
- Latest value is replayed to new subscribers.
- Upstream work starts with the first subscriber.
- Upstream work stops after the last subscriber drops.
- Cancellation remains owned by subscription handles.

The current implementation matches the core of this:

- `shell/core/src/source/support.rs` has `shared_source(kind, path, create)`.
- The key includes primitive kind, `TypeId`, and path.
- `property`, `relation`, `children`, `children_events`, `node`, and `watch`
  use `shared_source`.
- `ShareReplayHub` replays the latest value to new subscribers and disconnects
  upstream on the last unsubscribe.
- `SHELL_CORE_SOURCE_TRACE` can log cache hit/miss, connect, subscribe,
  unsubscribe, and disconnect behavior.

Remaining decision gap: the cache keeps one hub per descriptor for the process
lifetime. That stops upstream work but does not evict inactive descriptor keys.
This may be fine for stable shell paths, but dynamic D-Bus object churn should
be measured.

### Path Normalization

The old audit found `../../` relation targets escaping the mount root. Current
code has moved:

- `LocusPath::new` now lexically normalizes `.` and `..`.
- `relation::read_relation` and watch event handling both use
  `watch_value_path`.
- Tests cover mount-absolute targets, logical absolute targets, and relative
  relation targets.

Remaining decision gap: source primitive functions accept `impl Into<PathBuf>`.
Calls that bypass `LocusPath` can still hand `shared_source` non-normalized
paths, so equivalent raw paths may not share. Either document "use `LocusPath`
for source descriptors" as a hard rule, or normalize inside the source
primitive boundary.

### D-Bus Layout

The latest path decision is documented in `AGENTS.md` and confirmed by the
locusfs D-Bus tests:

- Service root: `/dbus/<service>`.
- Object properties live under `/dbus/<service>/objects/...`.
- Callable methods live under `/dbus/<service>/methods/...`.
- Paths outside the configured ObjectManager root are namespaced under
  `_absolute`.
- Legacy `object`, `@properties`, `@methods`, and `@absolute` path assumptions
  should be removed.

Additional details from `../locusfs/plugins/dbus/src/state/test.rs`:

- A service root lists `objects` and `methods`.
- If an object path equals the configured ObjectManager path, its properties are
  directly under `/dbus/<service>/objects/<Property>`.
- If the ObjectManager path is `/`, object paths are exposed relative to
  `/dbus/<service>/objects` without `_absolute`.
- Property files use short names when unique and not conflicting with child
  object directory names; canonical `interface.Property` names are also exposed.
- Method files mirror the object tree under `methods`; the method file itself
  is callable. There is no trailing `/call` path in the latest layout.

## Docs Needing Updates

### `SOURCE_API.md`

Recommended updates:

- Add an "Implementation status" note that primitive locusfs sources already
  use descriptor-keyed share/replay in `shell_core::source`.
- Clarify that sharing currently applies to primitive backend descriptors, while
  higher-level derived Observables may still duplicate aggregation work unless
  generated/source policy adds a stable key.
- State whether descriptor paths are normalized at the source boundary or by
  `LocusPath` convention.
- Add a short D-Bus path subsection with examples:
  - UPower battery object:
    `source::root().child("dbus/upower/objects/devices/battery_BAT1")`
  - PowerProfiles root object property:
    `source::root().child("dbus/powerprofiles/objects").observe_prop("ActiveProfile")`
  - BlueZ method:
    `source::root().child("dbus/bluez/methods/org/bluez/hci0/dev_XX/Connect")`
- Explicitly say that legacy `@properties`, `@methods`, and `/call` method
  suffixes are not part of the current D-Bus layout.

### `PLAN.md`

Recommended updates:

- Mark primitive descriptor-keyed sharing as implemented, pending validation.
- Keep derived/generated descriptor sharing as future work only where stable
  keys exist.
- Replace the current "Next Concrete Step" with:
  1. Update stale docs/audits for implemented sharing and path normalization.
  2. Migrate shell D-Bus consumers to `/objects` and `/methods`.
  3. Add tests and runtime validation for sharing and D-Bus layout.
- Add one sentence that `rsynapse-shell/config/locusfs/config.toml` defines
  service `local_id`s, and those IDs drive `/dbus/<local_id>/objects|methods`
  paths.

### `docs/audits/shell-watch-paths.md`

This audit should stay as historical evidence, but it needs a status banner.

Recommended banner:

- Historical audit from 2026-06-22.
- Shell-side path normalization and primitive descriptor sharing have since
  been implemented.
- Still-active concerns: descriptor cache retention, derived aggregate fanout,
  nested source lifecycle verification, and locusfs/FUSE shutdown behavior.

Specific stale sections:

- "Source primitives are cold and explicitly not shared" is no longer true.
- "LocusPath does not normalize lexical `..`" is no longer true.
- "relation normalizes watch events differently from initial read" is no
  longer true.
- Line references to `source/mod.rs` TODOs are stale.

Keep the validation checklist, but revise FD-sharing expectations to use
`SHELL_CORE_SOURCE_TRACE` and to distinguish primitive watch sharing from
derived aggregate duplication.

### `docs/audits/statusnotifier-dbusmenu.md`

Recommended updates:

- Mark old `/dbus-service/powerprofiles/object/@/ActiveProfile` as a historical
  path from the observed logs/code, not the current target.
- Add the current equivalent:
  `/dbus/powerprofiles/objects/ActiveProfile`.
- Keep the locusfs runtime, zbus, shutdown, and FUSE write findings active
  unless fixed in the adjacent locusfs repo.

### `docs/audits/locusfs-fuse-hang.md`

Recommended updates:

- Add a status note that shell path normalization has been implemented in
  `LocusPath` and relation target resolution.
- Keep the FUSE invalidation, plugin shutdown, and zbus runtime findings active.
- Keep the old `../../` path as historical evidence, but do not present it as
  the current shell implementation state without revalidation.

### `ags-migration/missing-shell-features/shared-source-fanout-keys.md`

Recommended replacement direction:

- Gap is partially closed by `shell_core::source::shared_source`.
- Remaining work is validation, descriptor cache policy, derived/generated
  source sharing, and documentation of what "equivalent source" means.
- Remove `ObservableSource` wording; that abstraction is explicitly out of the
  target design.

### `rsynapse-shell/config/locusfs/config.toml`

The config is mostly fine. It registers the D-Bus services with `local_id`s:

- `agentdbus`
- `upower`
- `networkmanager`
- `bluez`
- `powerprofiles`

No legacy filesystem path strings are encoded in the config. Optional doc
comment: note that each `local_id` maps to `/dbus/<local_id>/objects` and
`/dbus/<local_id>/methods`.

### Secondary Migration Docs

Search also found stale path text outside the requested core files:

- `ags-migration/migration/widgets/system-indicators.md` still mentions
  `dbus-service/upower/object/battery_BAT1/`.
- `ags-migration/migration/widgets/agent-approvals.md` still frames
  ObjectManager and method support as missing provider features. The wording
  should be updated to "consume the generic locusfs D-Bus projection" unless a
  separate shell helper is still desired.

## Current Consumer Path Drift

These code call sites still encode the legacy D-Bus layout and should be
covered by future docs/tests before implementation:

- `rsynapse-shell/src/widgets/bar/battery.rs`
  - current: `dbus/upower/object/devices/battery_BAT1/@properties`
  - target: `dbus/upower/objects/devices/battery_BAT1`
- `rsynapse-shell/src/widgets/bar/power_profile.rs`
  - current: `dbus/powerprofiles/object/@/@properties`
  - target: `dbus/powerprofiles/objects`
  - write target: `dbus/powerprofiles/objects/ActiveProfile`
- `rsynapse-shell/src/widgets/bar/network/mod.rs`
  - current roots: `dbus/networkmanager/object`,
    `dbus/networkmanager/object/Devices`
  - target roots: `dbus/networkmanager/objects`,
    `dbus/networkmanager/objects/Devices`
  - `properties(object)` should become identity for property reads.
- `rsynapse-shell/src/widgets/bar/bluetooth/source.rs`
  - current BlueZ root: `dbus/bluez/object/org/bluez/hci0`
  - target BlueZ root: `dbus/bluez/objects/org/bluez/hci0`
  - current method path: `object/@methods/<Method>/call`
  - target method path: corresponding `dbus/bluez/methods/.../<Method>` file.
- `rsynapse-shell/src/widgets/bar/project_label/source/agent.rs`
  - current sessions root: `dbus/agentdbus/object/sessions/codex`
  - target sessions root: `dbus/agentdbus/objects/sessions/codex`
  - `session.child("@properties")` should become `session`.
- `rsynapse-shell/src/widgets/bar/window_tile/agent/source/actual.rs`
  - same AgentDBus sessions and `@properties` drift.

## API Review

The Observable-first API is clean and keeps implementation details mostly out
of consumers. Consumers see `Observable<T>`, `LocusPath`, and Rx operators
rather than watch handles or custom source traits.

The current D-Bus path surface is not clean enough. Raw strings such as
`"dbus/networkmanager/object"` and helpers like `object.child("@properties")`
spread the old layout across widget sources. A tiny path-construction helper
would make the layout easier to migrate and harder to regress.

Decision to make before implementation:

- Keep D-Bus path helpers private in `rsynapse-shell` until duplication proves
  they belong in `shell_core::source`.
- Or add shell-core path helpers such as `dbus_objects(service)`,
  `dbus_methods(service)`, and `dbus_object_path(service, dbus_path,
  object_manager_path)` while still avoiding any D-Bus provider/runtime API.

The conservative choice is to start with private helper functions in affected
consumer modules, then promote only if several modules need exactly the same
logic.

## Redundancy Review

Redundant concepts now visible:

- Repeated `properties(object) -> object.child("@properties")` helpers.
- Repeated D-Bus service/object path constants.
- Two separate AgentDBus session aggregate implementations in project label and
  window tile sources.
- Repeated object-path mapping logic for NetworkManager and likely future
  AgentDBus approval sources.

Refactor direction:

- Eliminate `@properties` helpers during D-Bus layout migration.
- Add small local helpers for service roots and method roots.
- Revisit shared AgentDBus session aggregation after path migration. Primitive
  source sharing prevents duplicate watches, but the aggregate CPU/allocation
  work can still be duplicated across widgets.

## Performance And Concurrency Review

Implemented sharing should reduce duplicate FUSE watch/read descriptors for
equivalent primitive source paths. The design is good for the high-fanout source
shape in network, Bluetooth, audio, systray, and agent state.

Remaining risks:

- The descriptor cache retains hubs forever. Dynamic object churn can become a
  process-lifetime memory footprint even after upstream watches disconnect.
- Equivalent raw `PathBuf`s may not share if they bypass `LocusPath`
  normalization.
- Derived aggregate functions still rebuild Rx chains and may duplicate CPU work
  even though primitive watches are shared.
- Tokio abort still cannot unwind a worker blocked in uninterruptible FUSE
  kernel I/O. Docs should not imply source cancellation fully solves locusfs
  daemon shutdown hangs.
- Object tree directory reads under the new D-Bus layout may mix object child
  directories and property files. Consumers need either file-type-aware listing,
  explicit object roots, or documented expectations from locusfs.

## Tidiness Review

The code/docs boundary is understandable, but the docs need status hygiene:

- Audits should be clearly marked historical vs active.
- `SOURCE_API.md` should be the central source API contract.
- `PLAN.md` should track roadmap status, not stale TODOs.
- Migration feature notes should not reference removed abstractions like
  `ObservableSource`.
- D-Bus path examples should be centralized so widgets do not relearn the
  latest locusfs layout from old audits.

## Best Practices Review

Good decisions to keep:

- Use Rx operators for composition.
- Keep direct `locusfs-watch` access private to shell-core source primitives.
- Do not fix source correctness with debounce/throttle/sleep.
- Do not reintroduce provider crates or `ObservableSource`.
- Keep D-Bus transport/projection in locusfs; shell consumes filesystem paths.

Recommended best-practice addition:

- Add grep-style documentation tests/checks for forbidden legacy path tokens
  outside historical audit blocks: `dbus-service`, `/object`, `@properties`,
  `@methods`, `@absolute`, and `/call` for D-Bus methods.

## Domain-Specific Path Layout

Current target rules for shell docs:

- `/dbus/<service>/objects` is the object/property tree.
- `/dbus/<service>/methods` is the method call tree.
- ObjectManager-relative object paths are exposed relative to `objects`.
- Root ObjectManager services such as BlueZ expose `/org/bluez/...` as
  `objects/org/bluez/...`.
- Objects outside the ObjectManager root are exposed under
  `objects/_absolute/...`.
- Properties are files directly under the object path. Short names are exposed
  when unambiguous; canonical `interface.Property` names are available.
- Methods are files directly under the mirrored method path. The method file is
  the callable endpoint.
- Legacy synthetic directories are absent by design.

Important open design point:

- Object directories can list both child object directories and property files.
  If shell code needs "child objects only", the docs must state whether
  `children()` is enough, whether property files are filtered by locusfs, or
  whether shell-core needs a typed/file-kind children primitive.

## Concrete Refactor Plan

1. Update docs before code:
   - Add status notes to the three audits.
   - Update `SOURCE_API.md` with implemented sharing semantics and current
     D-Bus layout examples.
   - Update `PLAN.md` roadmap status.
   - Rewrite `shared-source-fanout-keys.md` as "partially implemented,
     validation remains".
   - Either create `docs/plans/config/README.md` for source/path decisions or
     remove references to that nonexistent area.

2. Add/settle D-Bus path helper shape:
   - Document whether helpers live in `rsynapse-shell` or `shell_core::source`.
   - Cover object roots, manager-root object properties, `_absolute`, and
     method paths.
   - Avoid a D-Bus provider/runtime abstraction.

3. Migrate consumer path strings:
   - Battery, PowerProfiles, NetworkManager, BlueZ, AgentDBus project label, and
     AgentDBus window tile sources.
   - Replace `@properties` with direct object property reads.
   - Replace method `/call` paths with direct method files under `/methods`.

4. Re-check aggregate duplication:
   - Only after path migration, decide whether the duplicate AgentDBus session
     aggregate should move to one local shared source module.
   - Measure before adding higher-level descriptor caches.

5. Validate against running locusfs:
   - Use `SHELL_CORE_SOURCE_TRACE=1` to verify one upstream connection per
     primitive descriptor.
   - Use `lsof` or locusfs metrics to confirm watch descriptor counts stabilize.
   - Inspect actual `/run/user/$UID/locusfs/dbus/<service>` trees for property
     and method names.

## Tests To Add Or Run

No tests were run during this read-only review.

Recommended focused tests:

- `shell-core` source sharing:
  - same kind/type/path calls share one upstream create/connect;
  - last unsubscribe disconnects upstream;
  - later subscriber reconnects and receives latest only when available;
  - normalized equivalent paths either share or the public rule forbids raw
    unnormalized `PathBuf`s.
- `shell-core` path tests:
  - relation targets still normalize for mount-absolute, logical absolute, and
    relative paths;
  - attempted mount escape is rejected or normalized according to the chosen
    policy.
- D-Bus path helper tests:
  - object-manager root object property path;
  - root ObjectManager service path such as BlueZ;
  - outside-manager `_absolute` path;
  - method file path without `/call`;
  - property name collision where canonical interface-qualified names are used.
- Consumer source tests:
  - `networkmanager_object_path` maps a D-Bus object path to
    `/dbus/networkmanager/objects/...`;
  - BlueZ connect/disconnect write paths map from object paths to `/methods`;
  - Agent session roots use `/objects/sessions/codex`.
- Documentation checks:
  - grep for forbidden legacy layout tokens outside historical audit sections.
- Manual integration:
  - Start locusfs with `rsynapse-shell/config/locusfs/config.toml`.
  - Start shell with `SHELL_CORE_SOURCE_TRACE=1`.
  - Open network, Bluetooth, power profile, agent, and systray views.
  - Confirm no `Too many open files`, no `path escapes mount root`, and no
    legacy D-Bus path reads.

## Validation Questions

1. Should descriptor cache hubs be evicted after the last subscriber drops, or
   is process-lifetime retention acceptable for shell sessions?
2. Should `shared_source` normalize paths internally, or should `LocusPath` be
   the only supported descriptor construction path?
3. Should derived source functions get descriptor-keyed sharing, or is primitive
   backend sharing enough for the current performance target?
4. Should shell-core expose D-Bus path helpers, or should `rsynapse-shell`
   keep them private until another consumer needs the same helpers?
5. How should shell code distinguish child object directories from property
   files under `/dbus/<service>/objects/...` when it needs object collections?
6. Should docs preserve old audit paths exactly as historical evidence, with
   status notes, or rewrite examples to current paths and move old paths into
   quotes/log blocks only?
7. Should consumers prefer canonical `interface.Property` names for D-Bus
   properties to avoid future short-name collisions, even when short names are
   currently unique?
8. What is the intended write contract for D-Bus method files: write queued,
   write completed after D-Bus reply, or method-specific status exposed
   separately?
