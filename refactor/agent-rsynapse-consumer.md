# rsynapse-shell consumer source functions and widgets review

Date: 2026-06-26

Scope: `rsynapse-shell/src/widgets/**`, `rsynapse-shell/config/locusfs/config.toml`, and the shell-core source/list APIs that determine whether consumer duplication is real upstream work or just duplicated derived composition.

No source code was edited in this pass.

## Required Context Read

- `AGENTS.md`
- `PROJECT.md`
- `PLAN.md`
- `SOURCE_API.md`
- `rsynapse-shell/src/widgets/AGENTS.md`
- Relevant shell-core source/list files:
  - `shell/core/src/source/mod.rs`
  - `shell/core/src/source/support.rs`
  - `shell/core/src/locus_path/mod.rs`
  - `shell/core/src/list/box_container.rs`
- Adjacent locusfs D-Bus layout evidence:
  - `../locusfs/plugins/dbus/src/state/test.rs`
  - `../locusfs/plugins/dbus/src/state.rs`

## Executive Summary

The highest-priority correctness issue is stale D-Bus path layout usage in `rsynapse-shell`: consumers still assume `dbus/<service>/object`, `@properties`, and `@methods/.../call`, but the current locusfs D-Bus plugin exposes service roots as `objects` and `methods`, puts object properties directly under `objects/...`, exposes callable methods directly under `methods/.../<method>`, and uses `_absolute` for object paths outside the configured ObjectManager root.

The highest-priority performance/design issue is not duplicated primitive locusfs reads. Shell-core already shares primitive path-backed `property`, `children`, `relation`, `node`, and `watch` sources by kind, path, and output type. The remaining duplication is mostly derived composition: multiple row-level source functions rebuild the same "all windows", "all AgentDBus sessions", BlueZ, UPower, and PipeWire summaries, then filter them per row or per popover. That creates avoidable `combine_latest_vec`, sorting, allocation, and subscription churn, even though primitive watch/read work is mostly shared.

The recommended split is:

- Put generic descriptor-keyed derived source sharing in `shell-core::source`, not local `OnceLock` caches in `rsynapse-shell`.
- Keep concrete semantic UI snapshots, such as `WindowSnapshot`, `AgentSessionSnapshot`, `BluetoothDeviceSnapshot`, and `AudioSinkSnapshot`, in `rsynapse-shell` because they are consumer UI policy.
- Keep stable `LocusPath` values as list row identities until shell-core's list API supports keyed row updates. The current list reconciliation matches rows by full `Init: PartialEq`, so passing mutable snapshots as row init values would recreate rows on every snapshot change.

## API Review

### What is good

- Consumer source functions mostly return UI-facing view models rather than raw transport details, for example `battery_status() -> Observable<BatteryView>`, `network_status() -> Observable<NetworkView>`, and `project_label_vm(workspace) -> Observable<ProjectLabelVm>`.
- Widget models store plain values and bind through `#[source(...)]`, matching the Observable-first design in `SOURCE_API.md`.
- Most sources compose RxRust operators directly: `combine_latest!`, `switch_map`, `map`, `filter_map`, `distinct_until_changed`.
- Locusfs primitive access stays behind `shell_core::source` helpers in normal widget sources.

### API problems

1. D-Bus filesystem layout leaks everywhere as raw string constants and local helpers:
   - `rsynapse-shell/src/widgets/bar/battery.rs:4`
   - `rsynapse-shell/src/widgets/bar/power_profile.rs:5`
   - `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:12-13`
   - `rsynapse-shell/src/widgets/bar/network/mod.rs:11-12`
   - `rsynapse-shell/src/widgets/bar/project_label/source/agent.rs:9`
   - `rsynapse-shell/src/widgets/bar/window_tile/agent/source/actual.rs:10`

   This makes the D-Bus plugin's current implementation details part of each widget source. It also made the `object` to `objects` migration expensive because every widget encoded the old layout differently.

2. D-Bus action paths leak into view models:
   - `BluetoothDeviceView` stores `connect_path` and `disconnect_path` in `rsynapse-shell/src/widgets/bar/bluetooth/mod.rs:247-248`.
   - The row writes those paths directly in `rsynapse-shell/src/widgets/bar/bluetooth/mod.rs:184-191`.
   - `BluetoothStatusView` carries `power_path`, then `toggle_power` writes it in `rsynapse-shell/src/widgets/bar/bluetooth/mod.rs:199-213`.

   This is acceptable for a consumer crate, but it keeps UI component code coupled to filesystem call/write mechanics. A better boundary is a small action helper close to the source module, for example `connect_device(path)` and `set_adapter_power(path, bool)`, with the view model carrying a stable object identity rather than a prebuilt legacy call path.

3. The current `#[bind_list]` API limits source refactors:
   - Shell-core list reconciliation removes and re-appends widgets on every update, then reuses old row controllers only when `old_row.item == item` (`shell/core/src/list/box_container.rs:49-67`).
   - `C::Init: Clone + PartialEq` is the only identity (`shell/core/src/list/box_container.rs:14-18`).

   This means replacing list items with rich snapshots would cause row recreation whenever any snapshot field changes. Keep stable `LocusPath` row init values unless shell-core gains keyed row updates.

## Redundancy Review

### D-Bus path construction

Repeated old helpers and path constants should be replaced with one layout-aware helper:

- `object.child("@properties")` is duplicated in:
  - `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:198-200`
  - `rsynapse-shell/src/widgets/bar/network/mod.rs:154-156`
  - `rsynapse-shell/src/widgets/bar/project_label/source/agent.rs:103-108`
  - `rsynapse-shell/src/widgets/bar/window_tile/agent/source/actual.rs:58-64`
- `method_call_path` still builds `@methods/<method>/call` in `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:334-336`.
- `networkmanager_object_path` hand-rolls object path trimming in `rsynapse-shell/src/widgets/bar/network/mod.rs:95-105`.

Recommended shape:

```rust
struct DbusServicePath {
    local_id: &'static str,
    object_manager_path: &'static str,
}

impl DbusServicePath {
    fn objects(&self) -> LocusPath;
    fn methods(&self) -> LocusPath;
    fn object_path(&self, dbus_path: &str) -> Option<LocusPath>;
    fn object_relative(&self, relative: impl AsRef<Path>) -> LocusPath;
    fn method_for_object_path(&self, dbus_path: &str, method: &str) -> Option<LocusPath>;
}
```

This helper should start in `rsynapse-shell` because concrete configured service IDs are consumer policy. If another consumer needs it, move only the generic path builder into `shell-core`.

### Window graph snapshots

The same `window` collection is rebuilt in several places:

- `selected_workspace_windows()` scans all windows and reads `workspace-id`, `column`, `row`, `id`: `rsynapse-shell/src/widgets/bar/workspaces.rs:62-112`.
- Project label fallback scans all windows and reads `workspace-id`, `column`, `row`, `id`, `app-id`: `rsynapse-shell/src/widgets/bar/project_label/source/workspace_fallback.rs:26-61`.
- Project label agent state scans all windows and reads `workspace-id`, `id`: `rsynapse-shell/src/widgets/bar/project_label/source/agent.rs:60-86`.
- Each window tile row reads `id`, `title`, `app-id`, `selected`, and `urgent`: `rsynapse-shell/src/widgets/bar/window_tile/source.rs:31-64`.

Primitive reads are shared by shell-core, but each derived function rebuilds dynamic vector sources, sorts, filters, and allocates independently.

Recommended source model:

- Add a bar-local `WindowSnapshot` source with stable fields used by multiple widgets:
  - `path`
  - `id`
  - `workspace_id`
  - `column`
  - `row`
  - `app_id`
  - `title`
  - `selected`
  - `urgent`
- Use shell-core derived sharing to make `all_window_snapshots()` one process-local shared observable.
- Keep row init values as `LocusPath` until keyed list row updates exist. Use snapshots for aggregate views such as selected-window list, project fallback, and workspace agent matching.

### AgentDBus sessions

Agent session state is duplicated across two modules:

- Project labels: `rsynapse-shell/src/widgets/bar/project_label/source/agent.rs:88-117`
- Window tiles: `rsynapse-shell/src/widgets/bar/window_tile/agent/source/actual.rs:43-78`

Both use the same legacy path and parse `WindowId`, `State`, and `RequiresAttention`; the window-tile copy additionally parses `ContextPct`.

Recommended source model:

- Create one `AgentSessionSnapshot` source under the bar/window-agent area or a bar-local source module.
- Include `window_id`, `state`, `requires_attention`, and `context_pct`.
- Let `workspace_agent_state(workspace)` and `agent_for_window(window)` map from the shared sessions source.
- Share the derived sessions collection with shell-core source sharing rather than a consumer-local `OnceLock`.

### Bluetooth and UPower

`bluetooth_status()` and `bluetooth_group_devices(group)` both scan BlueZ objects:

- Status summary: `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:46-67`
- Group detail source: `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:69-101`

`bluetooth_group_devices(group)` is called per popover group from `rsynapse-shell/src/widgets/bar/bluetooth/mod.rs:94-100`, so each mounted group can rebuild the same BlueZ and UPower device lists.

Recommended source model:

- Build one shared `bluetooth_snapshot()` or `bluetooth_devices()` derived observable.
- Derive `BluetoothView` and `Vec<BluetoothDeviceView>` group filters from it.
- Use the same D-Bus path helper for BlueZ objects, UPower objects, adapter writable properties, and method call files.

### Audio and OSD

The bar and OSD both observe the default sink:

- Bar audio status: `rsynapse-shell/src/widgets/bar/audio/source.rs:23-29` and `default_sink_view` at `rsynapse-shell/src/widgets/bar/audio/source.rs:53-74`
- OSD audio events: `rsynapse-shell/src/widgets/osd/source.rs:30-47`

Primitive default-sink property reads are shared. The repeated icon and level mapping can stay local, but a shared `default_sink_snapshot()` would reduce repeated composition if audio grows.

`audio_routes()` is also used by `AudioRoutePopover` (`rsynapse-shell/src/widgets/bar/audio/route_popover.rs:9-13`), so route list work starts when the popover component is mounted.

### Source error helpers

`source_error_count()` reimplements `source::error_count()`:

- Local map: `rsynapse-shell/src/widgets/bar/source_errors.rs:7-12`
- Shell-core helper: `shell/core/src/source/mod.rs:281-283`

Use `source::error_count()` directly and keep only `source_error_items()` local unless shell-core adds `error_items()`.

## Performance And Concurrency Review

### Existing primitive sharing

Shell-core already satisfies the main primitive sharing goal:

- `shared_source(kind, path, create)` keys by source kind, `TypeId`, and `PathBuf`: `shell/core/src/source/support.rs:115-144`.
- New subscribers receive the latest active value before subscribing to the subject: `shell/core/src/source/support.rs:229-259`.
- Upstream connects on first subscriber and disconnects when the last subscriber drops: `shell/core/src/source/support.rs:260-279` and `shell/core/src/source/support.rs:359-364`.
- Primitive property, children, relation, node, and watch sources all call `shared_source`.

Do not add consumer-local caches for primitive paths. They would duplicate shell-core policy and increase cache lifetime risk.

### Remaining derived duplication

The main remaining cost is repeated derived composition:

- Each workspace row starts `project_label_vm(workspace)`, which starts window fallback and workspace agent sources (`rsynapse-shell/src/widgets/bar/project_label/mod.rs:15-20`, `rsynapse-shell/src/widgets/bar/project_label/source.rs:37-43`).
- Each `workspace_agent_state` currently rebuilds all windows and all AgentDBus sessions for that workspace (`rsynapse-shell/src/widgets/bar/project_label/source/agent.rs:30-58`).
- Each window tile starts `agent_for_window(window)`, which rebuilds all AgentDBus sessions for that window (`rsynapse-shell/src/widgets/bar/window_tile/source.rs:41`, `rsynapse-shell/src/widgets/bar/window_tile/agent/source/actual.rs:26-41`).

For small lists this is tolerable. Under window/session churn, it creates avoidable allocation, sorting, and `switch_map` subscription churn.

### Custom stream cancellation concern

`rsynapse-shell/src/widgets/osd/source.rs` uses `Shared::<()>::from_stream_result(brightness_stream(...))` at lines `56-67`. This is a local custom watcher using `notify`.

This conflicts with the widget AGENTS guidance that consumer sources should not call Rx stream factories when a shell-core primitive should own the bridge (`rsynapse-shell/src/widgets/AGENTS.md:24-33`). It also misses shell-core's abortable stream wrapper. Shell-core explicitly has a test showing RxRust's `from_stream_result` does not drop a pending stream on unsubscribe (`shell/core/src/source/support.rs:972-997`), while shell-core's local wrapper does (`shell/core/src/source/support.rs:950-970`).

Recommended action:

- Expose a small public shell-core primitive for abortable `Stream<Item = Result<T, String>>` bridging, or add a specific file watch/property primitive.
- Keep brightness local as a domain source until locusfs exposes brightness, but do not bridge it through RxRust's raw `from_stream_result`.

### Cache lifetime risk

Current shell-core primitive sharing stores hubs in a process-global `HashMap` and does not evict descriptor keys (`shell/core/src/source/support.rs:153-156`). It clears latest values and disconnects upstream work when subscribers drop, but the descriptor entry remains.

That is probably acceptable for a bounded set of long-lived shell paths. If shell-core exposes derived sharing for dynamic keys such as window IDs, DBusMenu items, or MPRIS players, the design should avoid unbounded strong cache retention. Prefer weak hubs or an explicit cleanup path.

## Tidiness Review

- `rsynapse-shell/src/widgets/bar/bluetooth/source.rs` is 358 lines and exceeds the widget guidance to keep files under 300 lines (`rsynapse-shell/src/widgets/AGENTS.md:12-14`). Split it as part of the Bluetooth source refactor.
- Source modules are generally cohesive and close to their widgets, matching `rsynapse-shell/src/widgets/AGENTS.md`.
- Repeated comments about RxRust inference stability in AgentDBus sources (`project_label/source/agent.rs:69-70`, `window_tile/agent/source/actual.rs:52-53`) are symptoms that a shared source helper would be cleaner.
- `network/mod.rs` combines Relm4-facing view types, source composition, D-Bus object path mapping, and pure rendering decisions in one file. It is under 300 lines but would benefit from moving path mapping to the shared D-Bus helper and tests.

## Best Practices Review

- Continue using RxRust operators for source composition. The codebase is already aligned with `SOURCE_API.md`.
- Do not use timing hacks for source correctness. None were found in the locusfs consumers.
- Do not add `OnceLock<HashMap<... Observable ...>>` caches in `rsynapse-shell`. The right reusable abstraction is a shell-core source sharing primitive that can be tested once.
- Keep derived semantic sources in the consumer crate. Shell-core should not learn what "selected workspace windows", "AgentDBus session status", or "Bluetooth audio group" mean.
- Move generic mechanics to shell-core:
  - derived source share/replay/refcount helper;
  - abortable stream bridge;
  - possibly keyed list row updates if rows need rich snapshot init data without churn.

## Domain-Specific D-Bus Path Layout

Current locusfs D-Bus plugin evidence:

- Service roots expose `objects` and `methods`, not `object`: `../locusfs/plugins/dbus/src/state/test.rs:324-333`.
- Object properties are direct files under `objects/...`; `@properties` and `@methods` are intentionally absent: `../locusfs/plugins/dbus/src/state/test.rs:336-347`.
- Method call files are direct files under `methods/.../<method>`: `../locusfs/plugins/dbus/src/state/test.rs:349-356`.
- Outside ObjectManager paths are under `_absolute`: `../locusfs/plugins/dbus/src/state/test.rs:359-395`.
- Root ObjectManager services such as BlueZ expose object paths relative to `objects/`: `../locusfs/plugins/dbus/src/state/test.rs:397-433`.
- The path resolver maps direct method path children to the method `call` property internally: `../locusfs/plugins/dbus/src/state.rs:566-592`.
- If an object property name collides with a child object segment, the short property name is suppressed and canonical `interface.Property` remains available: `../locusfs/plugins/dbus/src/state.rs:674-702`.

Required rsynapse path migrations:

| Current path or helper | New path or helper |
| --- | --- |
| `dbus/upower/object/devices/battery_BAT1/@properties` | `dbus/upower/objects/devices/battery_BAT1` |
| `dbus/powerprofiles/object/@/@properties` | `dbus/powerprofiles/objects` |
| `dbus/bluez/object/org/bluez/hci0` | `dbus/bluez/objects/org/bluez/hci0` |
| `dbus/upower/object/devices` | `dbus/upower/objects/devices` |
| `dbus/networkmanager/object` | `dbus/networkmanager/objects` |
| `dbus/networkmanager/object/Devices` | `dbus/networkmanager/objects/Devices` |
| `dbus/agentdbus/object/sessions/codex` | `dbus/agentdbus/objects/sessions/codex` |
| `object.child("@properties").prop(name)` | `object.prop(name)` |
| `object.child("@methods").child(method).prop("call")` | matching `dbus/<service>/methods/<object-relative-path>/<method>` |
| old outside-manager absolute namespace, if any | `dbus/<service>/objects/_absolute/<absolute/path/segments>` |

`rsynapse-shell/config/locusfs/config.toml` already declares the required service `local_id` and `object_manager_path` values. The config is not stale by itself. The stale assumptions are in consumer source paths.

## Concrete Refactor Plan

### Phase 1: D-Bus path helper and correctness migration

1. Add a small rsynapse-local D-Bus path helper, likely under `rsynapse-shell/src/widgets/bar/dbus_path.rs` or a `widgets/bar/source/` submodule.
2. Encode the configured service local IDs and ObjectManager roots from `config/locusfs/config.toml`:
   - `agentdbus`: `/io/github/AgentDBus`
   - `upower`: `/org/freedesktop/UPower`
   - `networkmanager`: `/org/freedesktop/NetworkManager`
   - `bluez`: `/`
   - `powerprofiles`: `/net/hadess/PowerProfiles`
3. Replace all `dbus/<service>/object` constants with `objects`.
4. Remove all `@properties` helpers and observe properties directly on object paths.
5. Replace Bluetooth method paths with the `methods` tree and write to the method file itself.
6. Update `networkmanager_object_path` to use the helper and `_absolute` fallback when a D-Bus path is outside the configured ObjectManager path.

### Phase 2: Shared semantic sources in rsynapse-shell

1. Add one shared `all_window_snapshots()` source for aggregate consumers.
2. Keep row init as `LocusPath` for `WindowTile` and `ProjectLabel` until shell-core supports keyed row updates.
3. Refactor:
   - `selected_workspace_windows()` to filter/sort shared snapshots and emit stable `Vec<LocusPath>`.
   - project-label fallback to map shared snapshots instead of scanning all windows per workspace.
   - workspace agent state to use shared window snapshots.
4. Add one shared `agent_sessions()` source with full details and map both project-label state and window-tile state from it.
5. Refactor Bluetooth to build one shared BlueZ/UPower device snapshot and derive status plus group views from it.
6. Consider a shared PipeWire sink snapshot if audio source growth continues.

### Phase 3: Shell-core source sharing API

Add a public shell-core helper for derived observables, not just primitive path sources.

Required semantics:

- Keyed by caller-provided descriptor plus output `TypeId`.
- Replays latest active value to late subscribers.
- Connects upstream on first subscriber.
- Disconnects upstream on last subscriber.
- Avoids unbounded strong cache retention for dynamic keys.
- Is generic enough for consumer semantic sources without leaking rsynapse-specific types into shell-core.

Possible API:

```rust
pub fn share_latest<T>(
    key: impl Into<SourceDescriptor>,
    create: impl Fn() -> Observable<T> + Send + Sync + 'static,
) -> Observable<T>
where
    T: Clone + Send + 'static;
```

This can reuse the existing `ShareReplayHub` internally after making cache retention safe for dynamic descriptors.

### Phase 4: Abortable custom source bridge

1. Expose shell-core's abortable stream bridge or add a file-watch source primitive.
2. Convert OSD brightness from `Shared::<()>::from_stream_result` to shell-core-owned bridge semantics.
3. Keep the brightness source in OSD until locusfs exposes a normalized brightness node.

### Phase 5: List API follow-up

If the project wants row components to receive rich snapshot data instead of stable paths, add keyed list reconciliation first.

Possible shape:

```rust
pub trait ListItem {
    type Key: Eq + Hash + Clone;
    fn key(&self) -> Self::Key;
}
```

Then rows can be reused by key while receiving updates through component input. Without this, changing `C::Init` from `LocusPath` to a mutable snapshot risks row teardown on every property update.

## Test Plan

### D-Bus path helper tests

Add pure unit tests for:

- UPower battery object path maps to `dbus/upower/objects/devices/battery_BAT1`.
- PowerProfiles root object maps to `dbus/powerprofiles/objects`.
- BlueZ root ObjectManager maps `/org/bluez/hci0` to `dbus/bluez/objects/org/bluez/hci0`.
- NetworkManager object paths inside the manager map relative to `dbus/networkmanager/objects`.
- Paths outside the manager map under `_absolute`.
- Bluetooth method path maps an object path to `dbus/bluez/methods/.../Connect`.
- Property helper never emits `@properties` or `@methods`.

### Source pure transform tests

Add focused tests for pure transforms that will move during refactor:

- Window snapshot sorting and selected-workspace filtering.
- Project fallback icon/empty computation from window snapshots.
- Agent session matching by window ID.
- Workspace agent attention/working aggregation.
- Bluetooth device grouping and UPower battery matching.
- NetworkManager D-Bus path mapping and AP selection behavior.

### Shell-core sharing tests

If adding public derived sharing:

- Two subscribers to the same descriptor subscribe to one upstream.
- Late subscriber receives latest active value.
- Last unsubscribe disconnects upstream.
- Later subscriber reconnects upstream.
- Different output types with same textual descriptor do not collide.
- Dynamic descriptor entries do not stay strongly retained forever.

### OSD stream bridge tests

If exposing an abortable stream bridge:

- Pending stream is dropped on unsubscribe.
- Error events reach source error handling or caller error path.
- No raw RxRust `from_stream_result` remains in `rsynapse-shell` custom watchers.

### Regression commands

Run narrow tests first:

```bash
cargo test -p shell-core source
cargo test -p rsynapse-shell
```

Then broaden:

```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```

## Open Validation Questions

1. Should the generic D-Bus path builder move to shell-core now, or stay in rsynapse until a second consumer needs it?
2. Should shell-core's source cache retain descriptor entries forever after upstream disconnect, or should the derived-sharing work also change primitive source cache retention to weak/evictable hubs?
3. Is it acceptable for list rows to remain path-initialized and source-bound locally, or should keyed list updates become part of this pass so aggregate snapshots can drive row contents directly?
4. Should D-Bus action writes remain raw filesystem writes from consumer action handlers, or should shell-core provide an async command/write helper with consistent error reporting?
5. Should brightness remain a local `/sys/class/backlight` watcher after the abortable bridge refactor, or is exposing brightness through locusfs part of the same milestone?

