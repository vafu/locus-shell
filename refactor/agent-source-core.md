# shell/core Source Runtime Review

Timestamp: 2026-06-26T01:05:26-07:00

Unit: `shell/core/src/source/**`, `shell/core/src/locus_path/**`, source-facing macro/runtime call sites, source-related Cargo/docs, and consumer usages needed to understand sharing and D-Bus path migration.

This is a report-only pass. No source code was intentionally edited.

## Inputs Read

- Repository instructions and source API docs: `AGENTS.md`, `PROJECT.md`, `PLAN.md`, `SOURCE_API.md`.
- Module instructions: `shell/core/src/source/AGENTS.md`.
- Source runtime files: `shell/core/src/source/{mod.rs,support.rs,watch.rs,property.rs,relation.rs,node.rs,children.rs,children_events.rs,conversion.rs}`.
- Path builder: `shell/core/src/locus_path/mod.rs`.
- Runtime integration: `shell/core/src/app.rs`, `shell/core/src/model.rs`, `shell/macros/src/locus_bindings/expand.rs`, `shell/core/src/list/**`.
- Consumer stress points: `rsynapse-shell/src/widgets/bar/**/source*.rs`, `battery.rs`, `power_profile.rs`, `network/mod.rs`, `bluetooth/source.rs`, `workspaces.rs`, row components.
- D-Bus layout source of truth: `../locusfs/plugins/dbus/src/state.rs`, `../locusfs/plugins/dbus/src/state/test.rs`, `../locusfs/refactor/arbitration.md`, and `rsynapse-shell/config/locusfs/config.toml`.
- Existing notes: `docs/audits/shell-watch-paths.md`, `docs/audits/statusnotifier-dbusmenu.md`, `TODO.md`.

## Verification

- `cargo test -p shell-core`: passed, 31 tests.
- `cargo test`: passed, 31 `shell-core` tests, 25 `shell-macros` tests, 3 `shell-rx-macros` tests, 1 doctest.
- First sandboxed Cargo attempt failed because `/home/v47/proj/locus-shell/target/debug/.cargo-lock` is outside the writable sandbox; rerun was approved because this review target is outside the active writable root.

## Executive Summary

`shell-core::source` has already moved past the older audit state: primitives are no longer purely cold. The current code has a process-global cache in `support::shared_source`, keyed by `(kind, TypeId, raw PathBuf)`, and a custom `ShareReplayHub` that starts upstream work on first subscription, replays the latest value to later active subscribers, and disconnects upstream work when the last subscriber drops.

That is the right direction, but it is only a partial sharing strategy. Equivalent source work is still multiplied because descriptors are private and implicit, paths are compared as raw `PathBuf`s after public entry, properties are keyed by decoded Rust type instead of one raw upstream property descriptor, derived semantic graphs are rebuilt independently by several consumers, and dynamic `children().switch_map(combine_latest_vec(...))` flows tear down and rebuild child subscriptions on every snapshot. Leaf watch file descriptors should usually be shared today when the exact path, primitive kind, and value type match; higher-level observable objects, subscriptions, allocations, and dynamic list graph work are not.

The D-Bus migration is also concrete now. Current `locusfs` intentionally replaced legacy `object`, `@properties`, `@methods`, and `@absolute` paths with service-local `objects`, `methods`, and `_absolute` path trees. `rsynapse-shell` still has multiple hard-coded legacy paths and method call paths that will fail or miss data against the latest layout.

## Current Source Descriptors

There is no public source descriptor API today. Callers express sources as `LocusPath` plus methods returning `Observable<T>`.

Public source primitives in `shell/core/src/source/mod.rs`:

- `root() -> LocusPath`: mount root from `LOCUS_ROOT`, `LOCUSFS_ROOT`, runtime `XDG_RUNTIME_DIR/locusfs`, or `/tmp/rsynapse`.
- `watch(path) -> Observable<WatchEvent>`.
- `property<T>(path) -> Observable<Option<T>>`.
- `relation(path) -> Observable<Option<LocusPath>>`.
- `node(path) -> Observable<NodeState>`.
- `children(path) -> Observable<Vec<LocusPath>>`.
- `children_events(path) -> Observable<ChildrenEvent>`.
- `errors()` and `error_count()` for process-local source errors.
- composition helpers `once`, `combine_latest`, and compatibility `combine_latest_vec`.

Path-local methods on `LocusPath` are convenience wrappers:

- `as_watch`, `as_property`, `as_property_or`, `observe_prop`, `observe_prop_or`.
- `as_relation`, `as_relation_or`, `observe_rel`, `observe_rel_or`.
- `as_node`, `as_children`, `as_children_events`.

Private descriptor shape in `support.rs`:

```rust
struct SourceKey {
    kind: &'static str,
    type_id: TypeId,
    path: PathBuf,
}
```

Kinds currently used by primitives:

- `watch`
- `property`
- `relation`
- `node`
- `children`
- `children_events`

This key is implementation-only, and because it stores the final `T` type id, two readers of the same property path with different decode types cannot share upstream read/watch work.

## Why Equivalent Observables Still Multiply

1. Macro output subscribes once per bound field.

`shell-macros/src/locus_bindings/expand.rs` generates one `let source: shell_core::source::Observable<#ty, _> = #source; ... subscribe(...)` block per field. This is conceptually fine; the macro should not be responsible for deduplicating arbitrary Rust expressions. It does mean all reuse must happen in source functions or the source registry.

2. Row components multiply semantic graphs.

`#[bind_list(..., row = ProjectLabel)]` and `WindowTile` launch one row component per item. Each row starts its own `project_label_vm(workspace.path.clone())` or `window_tile_vm(window.clone())` source tree. The GTK list host reuses row controllers by `Init` equality, so it is not recreating rows unnecessarily on stable lists, but each row still owns a separate semantic observable graph.

3. Shared leaf primitives do not share derived observable graphs.

Examples:

- `workspaces.rs` builds a `window` children snapshot and per-window properties for `selected_workspace_windows()`.
- `project_label/source/workspace_fallback.rs` builds another `window` children snapshot and a similar per-window model.
- `project_label/source/agent.rs` builds `workspace_windows()` plus `agent_sessions()`.
- `window_tile/agent/source/actual.rs` builds a second `agent_sessions()`.

With the current cache, matching leaf paths such as `/window/<id>/workspace-id` or `/dbus/agentdbus/.../WindowId` should share upstream primitive watches when the exact key matches. The higher-level `children -> switch_map -> combine_latest_vec -> map` chains, intermediate vectors, sorting, closures, and subscriptions are still duplicated.

4. Dynamic list snapshots cause subscription churn.

The common pattern is:

```rust
source::root()
    .child("window")
    .as_children()
    .switch_map(|windows| {
        source::combine_latest_vec(windows.into_iter().map(window_entry).collect())
    })
```

Every emitted child snapshot creates a new vector of child observables and switches to a new combined observable. Leaf sharing limits duplicate file descriptors, but this still rebuilds the dynamic graph on broad invalidation. It also loses any derived per-child computation cache above the leaf primitive layer.

5. Cache keys are raw paths after public entry.

`LocusPath::new` now lexically normalizes `.` and `..`, which addresses an older audit finding. But public source functions still accept any `impl Into<PathBuf>` and pass that raw path into `shared_source`. Direct calls with equivalent unnormalized paths can miss the cache. Descriptor normalization should be enforced at the source boundary, not only by the path builder.

6. Property upstream sharing is typed too late.

`property<T>` caches by `TypeId::of::<Option<T>>()`. If one caller reads a path as `String` and another later reads the same file as `u32`, they open separate upstream property streams. The upstream filesystem work is identical: watch/read a text property file. Type-specific decoding should sit downstream of a raw shared property source.

7. The cache never evicts descriptor hubs.

`source_cache()` is a static `Mutex<HashMap<SourceKey, Box<dyn Any + Send>>>` containing strong `Arc<ShareReplayHub<T>>` values. The hub disconnects upstream work when subscriber count reaches zero, but the registry retains every descriptor ever seen for the lifetime of the process. Transient windows, access points, tray items, DBusMenu rows, and AgentDBus sessions can grow this map indefinitely across UI churn.

8. D-Bus path churn makes equivalent intent non-equivalent keys.

Legacy constants still use `dbus/<service>/object`, `@properties`, and `@methods`. Against the new layout, attempted fixes are likely to mix old and new strings unless path construction is centralized. That would create different source keys for the same intended service/object/property during migration.

## API Findings

- The public `Observable<T, E = String>` alias keeps the user-facing API Observable-first as required by `SOURCE_API.md`. It also exposes RxRust's `SharedBoxedObservable` shape directly through the alias, which is acceptable for now because `PLAN.md` explicitly says source composition should use RxRust operators.
- Public docs are thin for the important lifecycle contract. `source/mod.rs` documents primitive purpose but not descriptor sharing, replay behavior, when upstream starts/stops, or how errors terminate/restart a source.
- `SOURCE_API.md` says "Items carry `Result<T, E>` internally"; the implementation instead uses the Rx error channel and macro-generated `on_error` to turn terminal errors into model messages. A hard primitive error closes the shared subject, records an error, and relies on later subscriptions to recreate the subject. Decide whether the target contract is value-level errors or Rx terminal errors; do not leave both descriptions active.
- `LocusPath` is intentionally generic and independent from `source`, which is consistent with `PROJECT.md`. However, `prop` and `rel` are just `child` aliases. That keeps the API small, but it means source descriptors do not preserve whether a path is intended as a property, relation, object tree, or method file until a source primitive is chosen.
- There is no typed helper for the latest generic D-Bus path layout. That is probably correct for `shell-core`; the core boundary says generic Locus path composition belongs there, not service-specific D-Bus policy. `rsynapse-shell` should add a small local path helper for its configured services. If a second consumer needs the same logic, promote it deliberately.
- `root()` has both `LOCUS_ROOT` and legacy `LOCUSFS_ROOT`, plus runtime auto-detection. That is practical. Descriptor keys should still be based on the resolved root path and lexically normalized before caching.

## Redundancy Findings

- `property`, `node`, `children`, and `children_events` each carry a similar state-machine pattern: open target or parent, initial read, watch loop, ancestor filtering, reopen target when it appears, stop on hard error. This is real duplication, but it is not yet harmful enough to justify a large abstraction. A small reusable `TargetOrParentWatchStream` helper would be worthwhile only if it reduces the current copy-paste without hiding per-primitive event semantics.
- Consumer modules duplicate semantic graph queries:
  - window list snapshots: `workspaces.rs`, `project_label/source/workspace_fallback.rs`, `project_label/source/agent.rs`.
  - AgentDBus sessions: `project_label/source/agent.rs` and `window_tile/agent/source/actual.rs`.
  - D-Bus object property path helper: `network`, `bluetooth`, `agent`, `battery`, and `power_profile`.
- `combine_latest_vec` remains as a compatibility spelling over `combine_latest`; it should be removed after call sites migrate if the compatibility name is no longer useful.
- The consumer-level repeated source functions should not move into `shell-core`. They are `rsynapse-shell` view-model composition, not generic framework behavior. A local `rsynapse-shell/src/widgets/bar/source` or `rsynapse-shell/src/sources` module is the right first home.

## Performance And Concurrency Findings

- Good: current `ShareReplayHub` implements the core desired lifecycle: connect on first subscriber, replay latest to active subscribers, disconnect on last unsubscribe. Existing tests cover last-subscriber disconnect and keeping upstream alive while another subscriber remains.
- Risk: `ShareReplayLatest::subscribe` clones `latest`, drops the state lock, emits `latest` directly to the observer, then subscribes that observer to the subject. An upstream emission between the clone and subject subscription can be missed by the new subscriber. This is a small but real race in a cross-thread source runtime.
- Risk: the `connecting` path assumes upstream subscription construction is not synchronously reentrant in surprising ways. Most current filesystem streams spawn async tasks, so this is probably fine today, but `shared_source` is generic and tests should cover synchronous immediate emit/complete cases if the helper stays generic.
- Risk: registry access is one global `Mutex<HashMap<...>>`. Subscription creation is not the hottest path compared with watch/read I/O, so this is acceptable now. Do not add a dependency like `dashmap` unless contention is measured.
- Risk: every stream source uses `tokio::spawn`. Relm4 currently brings Tokio runtime features into the dependency graph, and workspace tests pass, but `shell-core::source` has an implicit runtime requirement. Public docs or `ShellApp` should own that requirement explicitly.
- Risk: `JoinHandle::abort` is cooperative at the Rust task boundary, not a guarantee against a kernel/FUSE D-state wait. This matters because the previous audit observed a shell worker blocked in a FUSE symlink read. Sharing reduces pressure, but source runtime cancellation cannot be the only shutdown story.
- Risk: `open_target_or_parent` can subscribe to broad ancestors for missing targets. The ancestor event filter reduces unnecessary rereads, but many missing children under the same broad path can still create many ancestor watchers unless the descriptor registry shares those ancestor watches too.
- Allocation churn: `children()` rereads, filters, sorts, and allocates a full `Vec<LocusPath>` on every relevant change; dynamic consumers then allocate `Vec<Observable<T>>`, `Vec<Option<T>>`, and final view vectors. This is acceptable for small lists but should be watched for NetworkManager APs and DBusMenu trees.

## Best-Practice Notes

- Keep using RxRust operators for source composition. The custom runtime code should stay limited to bridging external filesystem/watch APIs into Observables and implementing descriptor-keyed sharing that RxRust does not provide directly with the required ref-count/disconnect semantics.
- Prefer descriptor structs/enums over stringly cache keys. A typed descriptor makes the sharing boundary testable, documentable, and easier to extend without leaking implementation details.
- Prefer raw upstream property sharing plus downstream typed decode. It matches the filesystem contract and avoids duplicating watch/read work just because two consumers decode differently.
- Use `Weak` cache entries or explicit idle eviction instead of a permanent strong global registry. That preserves sharing while avoiding lifetime leaks for transient graph paths.
- Do not put D-Bus-specific service constants or object-manager policy into `shell-core`. Keep that in `rsynapse-shell` until more consumers prove it belongs in a shared crate or a generated locusfs path helper.

## Latest D-Bus Path Layout

Current `locusfs` D-Bus state exposes a configured service node at:

```text
/dbus/<service-local-id>
```

The service node lists:

```text
/dbus/<service>/objects
/dbus/<service>/methods
```

Object property files live directly under `objects`. Callable methods live directly under the parallel `methods` tree as write-only files; callers write to the method file itself, not to `@methods/<method>/call`.

Object path display rules from `../locusfs/plugins/dbus/src/state.rs`:

- If object path equals the configured ObjectManager path, it is the root object and its properties live directly under `objects/`.
- If ObjectManager path is `/`, object paths are exposed without the leading slash, for example `/org/bluez/hci0` becomes `objects/org/bluez/hci0`.
- If object path is under the ObjectManager path, that prefix is stripped.
- Otherwise the object is outside ObjectManager and lives under `_absolute/...`.

Current `rsynapse-shell/config/locusfs/config.toml` service IDs:

- `agentdbus`: `io.github.AgentDBus`, ObjectManager `/io/github/AgentDBus`.
- `upower`: `org.freedesktop.UPower`, ObjectManager `/org/freedesktop/UPower`.
- `networkmanager`: `org.freedesktop.NetworkManager`, ObjectManager `/org/freedesktop/NetworkManager`.
- `bluez`: `org.bluez`, ObjectManager `/`.
- `powerprofiles`: `net.hadess.PowerProfiles`, ObjectManager `/net/hadess/PowerProfiles`.

Concrete shell replacements:

- Battery:
  - old: `dbus/upower/object/devices/battery_BAT1/@properties/<Property>`
  - new: `dbus/upower/objects/devices/battery_BAT1/<Property>`
- PowerProfiles root object:
  - old: `dbus/powerprofiles/object/@/@properties/ActiveProfile`
  - new: `dbus/powerprofiles/objects/ActiveProfile`
  - writes should target the same property file.
- NetworkManager device tree:
  - old: `dbus/networkmanager/object/Devices`
  - new: `dbus/networkmanager/objects/Devices`
  - device properties live at `dbus/networkmanager/objects/Devices/<id>/<Property>`.
  - access points from D-Bus object paths such as `/org/freedesktop/NetworkManager/AccessPoint/8` map to `dbus/networkmanager/objects/AccessPoint/8`.
- BlueZ:
  - old: `dbus/bluez/object/org/bluez/hci0`
  - new: `dbus/bluez/objects/org/bluez/hci0`
  - properties are direct children of object paths.
  - method calls should map from `objects/...` to `methods/.../<Method>`, for example `dbus/bluez/methods/org/bluez/hci0/dev_XX/Connect`.
- AgentDBus:
  - old: `dbus/agentdbus/object/sessions/codex` plus `@properties`.
  - new: `dbus/agentdbus/objects/sessions/codex`; session properties are direct children.
- Outside ObjectManager:
  - old: `@absolute` or raw absolute fallbacks.
  - new: `objects/_absolute/...` and `methods/_absolute/...`.

`children.rs` still filters `@...` entries, and tests mention `@methods`/`@properties`. That filter remains harmless, but the tests should be rewritten to describe "synthetic/private entries" rather than the retired D-Bus layout.

## Concrete Refactor Plan

### Phase 1: Make Descriptor Sharing Explicit Internally

Add a private or `pub(crate)` descriptor layer in `shell-core::source`:

```rust
enum SourceDescriptor {
    Watch { path: PathBuf },
    RawProperty { path: PathBuf },
    Relation { path: PathBuf },
    Node { path: PathBuf },
    Children { path: PathBuf },
    ChildrenEvents { path: PathBuf },
}
```

Rules:

- All public primitive functions normalize paths into descriptors before cache lookup.
- Descriptor labels are generated from the enum for tracing.
- `LocusPath` stays a generic path builder; descriptor construction belongs in `source`.
- Keep public source functions unchanged for compatibility.

### Phase 2: Replace Strong Permanent Cache Entries

Change the registry from strong `Arc` values to weak entries:

- Cache maps descriptor plus output type where needed to `Weak<ShareReplayHub<T>>`.
- On lookup, upgrade the weak entry; if upgrade fails, create a fresh hub.
- Optionally remove dead entries opportunistically on lookup/insert.
- Keep the current "clear latest on zero subscribers" behavior unless a conscious stale-replay policy is chosen.

This preserves stop-on-zero-subscribers while avoiding process-lifetime retention of every transient path.

### Phase 3: Share Raw Property Upstream

Split property observation:

- `raw_property(path) -> Observable<Option<String>>` owns watch/read work and is keyed only by normalized path.
- `property<T>` maps `raw_property(path)` through `FromLocusValue`.
- Decode errors should include the path and type context and should not poison the raw source for other decoders.

Optional later optimization: cache decoded typed properties by `(path, TypeId)` if typed decode itself becomes expensive. Do not let that duplicate raw watch/read work.

### Phase 4: Harden `ShareReplayHub`

Add focused tests before changing behavior:

- A second subscriber receives the latest active value without opening a second upstream subscription.
- Upstream disconnects exactly once when the last subscriber drops.
- A descriptor with no live observables can be evicted/recreated.
- No emission is missed when upstream emits while a new subscriber attaches.
- Synchronous upstream emit/complete does not leave stale `connecting` or `connection` state.
- Upstream errors reset state in a way that later subscribers can reconnect.

If the race fix becomes awkward with `SharedSubject`, consider replacing the subject-based hub with a small explicit observer list. That is more code, but it makes replay and subscription ordering easier to reason about than trying to coordinate direct observer replay with a subject.

### Phase 5: Reduce Consumer Semantic Duplication

Keep this in `rsynapse-shell`, not `shell-core`:

- Add shared source functions for window snapshots used by workspaces, project-label fallback, and agent matching. One candidate model can include path, workspace id, position, id, app id, selected, urgent, and title, with smaller map functions for each widget.
- Add one shared `agent_sessions()` source for both project labels and window tiles, with fields for window id, state, requires attention, and context pct.
- Keep widget-specific view-model formatting in the widget modules.

This reduces derived observable duplication and makes descriptor sharing less critical for common UI flows.

### Phase 6: Centralize D-Bus Path Construction In `rsynapse-shell`

Add a small consumer-local helper, for example `rsynapse-shell/src/locus_paths.rs` or `widgets/bar/source_paths.rs`:

```rust
fn dbus_service(service: &str) -> LocusPath;
fn dbus_objects(service: &str) -> LocusPath;
fn dbus_methods(service: &str) -> LocusPath;
fn dbus_object(service: &str, relative: impl AsRef<Path>) -> LocusPath;
fn dbus_method_for_object(object: &LocusPath, method: &str) -> LocusPath;
```

For known services, define constants or constructors that encode their ObjectManager rule:

- `upower_object("devices/battery_BAT1")`
- `powerprofiles_root_object()`
- `networkmanager_object_from_dbus_path("/org/freedesktop/NetworkManager/AccessPoint/8")`
- `bluez_object("org/bluez/hci0")`
- `agentdbus_object("sessions/codex")`

Then migrate all legacy call sites:

- `battery.rs`
- `power_profile.rs`
- `network/mod.rs`
- `bluetooth/source.rs`
- `project_label/source/agent.rs`
- `window_tile/agent/source/actual.rs`

Do not add legacy aliases in shell code. The locusfs arbitration decision says compatibility is not stable yet and the new layout should be preferred.

### Phase 7: Factor Watch State Machines Only After Tests

After descriptor and path migration tests are in place, consider extracting a small shared helper for target-or-parent watch streams. Keep event interpretation per primitive. Do not introduce a broad "source runtime" abstraction that competes with RxRust.

## Test Plan

Add or update focused tests:

- `locus_path`: keep lexical normalization tests; add source descriptor normalization tests for direct `PathBuf` primitive calls.
- `source::support`: test cache hit with a counted upstream factory; test weak eviction; test no duplicate upstream while two subscribers are active; test reconnect after zero subscribers.
- `source::property`: test one raw property upstream can feed two typed decoders without duplicate raw subscription, using a fake/test factory if the production function is too filesystem-bound.
- `source::support`: add replay race and synchronous upstream completion tests.
- `source::children`: update synthetic-entry test language away from D-Bus `@properties`/`@methods`; keep behavior if hidden entries remain possible.
- `rsynapse-shell` path helper tests:
  - PowerProfiles root object maps to `dbus/powerprofiles/objects`.
  - UPower battery maps to `dbus/upower/objects/devices/battery_BAT1`.
  - BlueZ root ObjectManager maps `/org/bluez/hci0` to `dbus/bluez/objects/org/bluez/hci0`.
  - NetworkManager AP object path maps to `dbus/networkmanager/objects/AccessPoint/8`.
  - Outside path maps under `_absolute`.
  - BlueZ method path maps `objects/...` to `methods/.../<Method>` with no trailing `call`.
- Consumer source smoke tests where pure enough:
  - `networkmanager_object_path`.
  - `method_call_path`.
  - `dbusmenu_local_id` remains percent-encoded through `LocusPath::encoded_child`.

Verification sequence after implementation:

1. `cargo test -p shell-core source::`
2. `cargo test -p rsynapse-shell`
3. `cargo test`
4. If runtime dependencies permit, start `rsynapse-shell` with `SHELL_CORE_SOURCE_TRACE=1` and confirm duplicate subscriptions hit the same descriptor and disconnect when rows disappear.
5. On a live session, compare `lsof`/watch fd growth before and after repeated shell restarts.

## Risks And Open Questions

- Should source errors be terminal Rx errors, or should primitives emit `Result<T, SourceError>` values as `SOURCE_API.md` says? Terminal errors are simpler but can silently stop updates until a resubscribe occurs.
- Should latest values be kept after the last subscriber drops? Current behavior clears latest so a later subscriber forces a fresh read. That avoids stale UI state and is the safer default.
- Should D-Bus path helpers stay consumer-local forever? Recommendation: yes until another shell consumer needs them.
- Should raw property sharing decode from plain text or from a typed `LocusValue` parser? Plain text matches the current FUSE read path; typed `LocusValue` would need a stable parser/format boundary.
- How much dynamic list churn is acceptable after leaf descriptor sharing is fixed? If NetworkManager APs or DBusMenu trees still churn heavily, the next step is keyed collection reconciliation at the Observable layer, not more widget-level caching.
- Does `ShellApp` need to own a Tokio runtime explicitly? Current tests pass, but the runtime requirement is implicit and should be documented or made explicit before `shell-core::source` is used by non-Relm4 binaries.

