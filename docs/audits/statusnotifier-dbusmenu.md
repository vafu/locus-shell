# StatusNotifier / DBusMenu / PowerProfiles Audit

Audit written: 2026-06-22 17:35 PDT.

Scope: previous boot logs from 2026-06-22, `../locusfs` plugin code for
StatusNotifier, DBusMenu, generic D-Bus writable properties, and the
`rsynapse-shell` systray/power profile consumers. No code changes were made.

## Summary

The hard hang was not a normal GTK/UI freeze. Kernel logs show both `locusfs`
and `rsynapse-shell` stuck in FUSE kernel waits, and systemd could not kill
`rsynapse-shell` with `SIGKILL`. The newest StatusNotifier/DBusMenu/PowerProfiles
work adds more D-Bus-driven graph mutation and writable-property paths, and the
audit found several concrete risks there.

Highest-priority findings:

1. `locusfs-statusnotifier` panics inside zbus with "no reactor running".
2. `rsynapse-shell` can become unkillable while blocked in FUSE `readlink`.
3. `locusfs` can block in FUSE reverse invalidation.
4. DBusMenu and PowerProfiles writes are synchronous FUSE writes that await
   D-Bus work.
5. DBusMenu only snapshots menu layout at registration, so action/layout updates
   are incomplete.
6. Stale tray nodes are a plugin-state cleanup problem, not something the shell
   should paper over.

## Timestamp Evidence

### zbus reactor panic

Previous boot, `locusfs.service`:

- `2026-06-22 12:55:22`: unnamed thread panicked in
  `zbus-5.16.0/src/abstractions/executor.rs:190:27` with:
  `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.
- `2026-06-22 12:55:22`: `fatal runtime error: Rust cannot catch foreign exceptions, aborting`.
- `2026-06-22 12:55:23`: `locusfs` dumped core with `SIGABRT`.
- `2026-06-22 13:02:15`: same zbus executor panic at `executor.rs:190:27`.
- `2026-06-22 13:02:16`: `locusfs` dumped core with `SIGABRT`.
- `2026-06-22 14:11:10`, `14:12:46`, `14:15:12`: same class of zbus panic,
  later at `executor.rs:63:27`.

The 14:12 run also logged:

- `2026-06-22 14:12:31`: `locusfs-statusnotifier: zbus executor task started`.
- `2026-06-22 14:12:46`: zbus panic with no Tokio reactor.

### repeated locusfs core dumps

`coredumpctl --no-pager list locusfs` showed many crashes on 2026-06-22:

- `12:45:01` `SIGSEGV`
- `12:49:49` `SIGSEGV`
- `12:50:15` `SIGSEGV`
- `12:50:18` `SIGSEGV`
- `12:55:23` `SIGABRT`
- `13:02:16` `SIGABRT`
- many more `SIGSEGV` entries between `14:07:15` and `16:36:42`

The stop/restart crashes often happen after:

- `locusfs-pipewire: pactl subscribe ended`
- `fusermount3: entry for /run/user/1000/locusfs not found in /etc/mtab`
- `locusfs.service: Main process exited, code=dumped, status=11/SEGV`

### hard hang / D-state evidence

Kernel logs from previous boot:

- `2026-06-22 17:18:46`: `tokio-rt-worker` from `locusfs` PID `642141`
  blocked for more than 122 seconds.
  Stack includes:
  `invalidate_inode_pages2_range -> fuse_reverse_inval_inode -> fuse_dev_write`.
- `2026-06-22 17:20:49`: same `locusfs` worker blocked for more than 245
  seconds in the same FUSE reverse invalidation path.
- `2026-06-22 17:20:49`: `tokio-rt-worker` from `rsynapse-shell` PID `696965`
  blocked for more than 122 seconds.
  Stack includes:
  `__fuse_simple_request -> fuse_readlink_folio -> fuse_symlink_read_folio`.
- `2026-06-22 17:22:52`: `locusfs` worker blocked for more than 368 seconds.
- `2026-06-22 17:22:52`: `rsynapse-shell` worker blocked for more than 245
  seconds.

Systemd user logs confirm that `SIGKILL` did not terminate the shell:

- `2026-06-22 17:18:18`: `rsynapse-shell-dev.service` stop timed out and sent
  `SIGKILL`.
- `2026-06-22 17:19:48`: processes still around after `SIGKILL`.
- `2026-06-22 17:22:48`: processes still around after final `SIGKILL`;
  unit entered failed mode.
- `2026-06-22 17:22:48`: unit process `696965 (rsynapse-shell)` remained
  running after the unit stopped.

### plugin reconnect loops during teardown

Near the reboot window, `locusfs` continued retrying external sources:

- `2026-06-22 17:22:40` onward:
  `locusfs-pipewire: failed to read PipeWire snapshot: ... Connection failure:
  Connection terminated`
- repeated `locusfs-pipewire: pactl subscribe ended`
- repeated `locusfs-niri: failed to reconnect event stream: I/O failed: No
  such file or directory (os error 2)`

These retries continued every 1-2 seconds until the boot ended at
`2026-06-22 17:24:10`.

### shell-side path error

`rsynapse-shell` logged:

- `2026-06-22 17:15:38`:
  `open relation watch ... failed: path escapes mount root` for a path with
  `/run/user/1000/locusfs/window/5/../../app-instance/.../agent-session`.

This is separate from StatusNotifier, but it is relevant because it means shell
sources can still build non-normalized relation paths that cross through `..`.

## Code Evidence

### StatusNotifier runtime

`../locusfs/plugins/statusnotifier/src/runtime.rs`:

- `StatusNotifierRuntime::start` calls `runtime.spawn_blocking`.
- Inside that blocking task it creates a new `Builder::new_current_thread()
  .enable_all()` runtime.
- `run_status_notifier_watcher` loops forever and calls
  `watch_status_notifier_bus`.
- Item watchers are spawned with the outer host `Handle` passed into the
  inner watcher logic.

This mixes three runtime contexts:

- host runtime handle from `PluginContext`
- a nested current-thread runtime created inside `spawn_blocking`
- zbus connection/executor internals

That is a strong root-cause candidate for the zbus "no reactor running" panic.
Even if most futures are polled inside one runtime, zbus may create/drop
executor internals on a thread without an entered Tokio runtime.

Shutdown also only aborts the outer `JoinHandle`. There is no explicit
cooperative cancellation path for the watcher loop, owned D-Bus names,
registered item tasks, or graph state cleanup.

### DBusMenu runtime

`../locusfs/plugins/dbusmenu/src/runtime.rs`:

- `DbusMenuRuntime::start` also uses `runtime.spawn_blocking`.
- It also creates a nested current-thread Tokio runtime.
- The watcher loops forever, retrying every second.
- It listens only for `StatusNotifierItemRegistered` and
  `StatusNotifierItemUnregistered`.
- It snapshots layout with `GetLayout`, then exposes nodes.

Missing pieces for complete DBusMenu behavior:

- no `LayoutUpdated` signal handling
- no `ItemsPropertiesUpdated` signal handling
- no submenu traversal/rendering; `DbusMenuItem::child_targets` currently
  returns an empty vector
- no action result feedback other than FUSE write success/failure

This explains menu actions/layout being partially functional: the UI can show a
snapshot and write `activate`, but menu state is not kept live.

### DBusMenu activation write

`rsynapse-shell/src/widgets/bar/systray/mod.rs`:

- each menu row is a `gtk::Button`
- clicking spawns a `std::thread` and calls
  `fs::write(item.prop("activate").as_path(), "true")`

`../locusfs/plugins/dbusmenu/src/provider.rs`:

- writable property accepts only `LocusValue::Bool(true)`
- then calls `enter_runtime(self.runtime.clone(), activate_item(target)).await`

`../locusfs/plugins/dbusmenu/src/runtime.rs`:

- `activate_item` creates a new D-Bus connection and sends
  `Event(item_id, "clicked", "", 0)` via `call_noreply`.

This design can work, but the FUSE write is only as safe as the provider call.
If zbus hangs or the runtime context is invalid, the write path can hold the
FUSE request.

### generic D-Bus writable property path

`../locusfs/fuse/src/fs/filesystem.rs`:

- property writes call `self.graph.set_property(&node, &key, value).await`
  directly inside the FUSE request.

`../locusfs/graph/src/graph/dynamic.rs`:

- `set_property` awaits the registered mutation provider, then emits a graph
  change.

`../locusfs/plugins/dbus/src/provider.rs`:

- `DbusProvider::set_property` resolves a writable property from state.
- It awaits `enter_runtime(self.runtime.clone(), set_dbus_property(...))`.
- Only after D-Bus returns does it update cached state.

`rsynapse-shell/src/widgets/bar/power_profile.rs`:

- clicking cycles profiles by spawning a thread and writing the next string to
  `/dbus-service/powerprofiles/object/@/ActiveProfile`.

Risk: PowerProfiles writes are synchronous FUSE writes that wait for system
D-Bus. Earlier observed shell logs included write errors such as "Software
caused connection abort" and "Transport endpoint is not connected" after
PowerProfiles clicks. A failed write should be reported, but must not crash or
deadlock the FUSE server.

### stale tray nodes

`../locusfs/plugins/statusnotifier/src/state.rs`:

- state emits `NodeRemoved` and `RelationRemoved` when item IDs disappear.
- stale cleanup depends on `remove_items_for_service`, which runs when
  `NameOwnerChanged` reports `new_owner == None`.

`../locusfs/plugins/dbusmenu/src/state.rs`:

- menu cleanup depends on `remove_service` from
  `StatusNotifierItemUnregistered`.

Observed stale tray nodes imply at least one of:

- unregistration/name-owner events are missed during watcher restart
- plugin restart begins with stale state instead of a fresh empty snapshot
- DBusMenu cleanup is keyed by the wrong service string
- state is not cleared on watcher failure/shutdown
- relation/node changes are emitted but the shell misses them due to FUSE watch
  hangs

Shell-side filtering would hide the symptom but would violate the desired
boundary. The plugin should own cleanup.

## Root Cause Candidates

### Candidate A: nested runtimes plus zbus executor context

Confidence: high.

Evidence:

- zbus explicitly panics with no Tokio reactor.
- StatusNotifier and DBusMenu use nested current-thread runtimes inside
  `spawn_blocking`.
- Other plugins such as `mpris`, `pipewire`, `niri`, and `dbus` use
  `locusfs_plugin_api::enter_runtime` around work spawned on the host runtime
  instead of building nested runtimes.

Recommended direction:

- Remove `spawn_blocking` plus nested runtime from StatusNotifier and DBusMenu.
- Spawn their watcher tasks on the host runtime, wrapped with `enter_runtime`
  if needed at plugin ABI boundaries.
- Keep all zbus connection, signal stream, and drop paths inside one entered
  Tokio runtime.

### Candidate B: plugin shutdown/retry loops ignore cancellation

Confidence: high.

Evidence:

- On session teardown, pipewire and niri retry loops kept running every
  1-2 seconds.
- Plugin shutdown mostly calls `JoinHandle::abort`.
- StatusNotifier/DBusMenu watcher loops sleep/retry forever and do not model a
  shutdown token.

Recommended direction:

- Add a plugin-level cancellation token or host shutdown signal.
- Make retry loops select on cancellation.
- On shutdown, clear plugin state and emit removals before the graph/FUSE layer
  disappears, or stop emitting graph changes after teardown begins.

### Candidate C: graph/FUSE invalidation can deadlock against client readlink

Confidence: high for the hang, exact internal cause needs FUSE audit.

Evidence:

- `locusfs` blocked in `fuse_reverse_inval_inode -> fuse_dev_write`.
- `rsynapse-shell` blocked in `fuse_readlink_folio`.
- Systemd could not kill the shell.

Recommended direction:

- Audit invalidation calls around symlink/relation updates and graph
  `emit_global_change`.
- Ensure reverse invalidation is not called while holding graph/watch locks that
  a readlink request needs.
- Add an integration test that reproduces concurrent relation invalidation plus
  client readlink/watch.
- Consider making graph event publication queue-based so plugin tasks do not
  synchronously block inside FUSE invalidation.

### Candidate D: writable D-Bus properties can block FUSE request handling

Confidence: medium-high.

Evidence:

- FUSE writes await `graph.set_property`.
- D-Bus property mutations await zbus `Set`.
- DBusMenu activation awaits zbus `Event`.
- User saw write failures around `ActiveProfile`.

Recommended direction:

- Keep FUSE write semantics bounded: add timeouts or an action-queue model for
  external D-Bus calls.
- If write returns before action completion, expose action status separately.
- Never let a D-Bus action hold locks required by reads/watches/invalidation.

This is not a UI debounce/time hack. It is a backend failure-boundary policy for
external IPC calls.

### Candidate E: DBusMenu state model is snapshot-only

Confidence: high for incomplete menu behavior.

Evidence:

- DBusMenu runtime handles StatusNotifier item register/unregister only.
- No `LayoutUpdated` or `ItemsPropertiesUpdated` watchers are present.
- Submenu child relations are stubbed as empty.

Recommended direction:

- Listen for DBusMenu layout/property update signals per menu endpoint.
- Refresh only the affected menu/item subtree.
- Implement child relations and menu item hierarchy before relying on complex
  tray menus.
- Add tests with a fake DBusMenu service.

## Recommended Fixes

1. Rewrite `statusnotifier` runtime to use the host Tokio runtime only.
   Do not create a nested runtime inside `spawn_blocking`.

2. Apply the same runtime cleanup to `dbusmenu`.

3. Add cooperative plugin shutdown:
   cancellation token, stop retry loops, abort/await child item watchers, clear
   graph state deterministically.

4. Harden FUSE graph invalidation:
   no reverse invalidation while holding locks that can be needed by lookup,
   readlink, or watch configuration.

5. Harden writable properties:
   make `dbus` and `dbusmenu` action writes bounded and non-deadlocking.
   Decide whether FUSE write means "queued" or "completed"; document the
   contract.

6. Complete DBusMenu protocol support:
   `LayoutUpdated`, `ItemsPropertiesUpdated`, submenu child relations, and
   activation feedback logging.

7. Fix stale tray nodes in plugins:
   clear StatusNotifier and DBusMenu state on watcher failure and on plugin
   shutdown; reconcile from a fresh snapshot after reconnect.

8. Fix shell path normalization separately:
   `LocusPath` relation targets should not produce mount-root-escaping paths
   with `../../`.

## Recommended Tests

### locusfs plugin tests

- Unit test StatusNotifier runtime startup under the host runtime without a
  nested runtime.
- Unit test DBusMenu runtime startup under the host runtime without a nested
  runtime.
- Fake StatusNotifier service: register item, update properties, unregister,
  assert node/relation add/remove events.
- Fake DBusMenu service: expose layout, emit `LayoutUpdated`, emit
  `ItemsPropertiesUpdated`, assert item properties and child relations update.
- DBusMenu activation test: write `activate=true`, assert `Event(clicked)` was
  received.
- PowerProfiles writable property test with a fake D-Bus properties service:
  write string property, assert D-Bus `Set` and graph cache update.

### FUSE/integration tests

- Concurrent relation retarget/readlink/invalidation stress test.
- FUSE write-to-writable-property with a mutation provider that blocks or
  errors; assert the mount remains responsive and clients are not left in
  unkillable waits.
- Plugin shutdown while clients hold watch/readlink handles; assert shutdown
  completes and clients receive an error instead of blocking forever.

### rsynapse-shell tests/manual checks

- Start shell with StatusNotifier and DBusMenu enabled; open Telegram tray menu.
- Click a menu action and verify action side effect.
- Close Telegram and assert tray item and DBusMenu nodes disappear from
  `/statusnotifier/item` and `/dbusmenu-*`.
- Toggle PowerProfiles repeatedly and verify `locusfs` stays mounted and shell
  remains killable.
- Run shell while restarting `locusfs`; verify source errors are logged but GTK
  remains responsive.

## Immediate Mitigation

Until the runtime and FUSE issues are fixed, the safest debugging profile is:

- disable `statusnotifier` and `dbusmenu` plugins when investigating global
  hangs;
- avoid rapid PowerProfiles writes while `locusfs` is unstable;
- stop `rsynapse-shell` before restarting `locusfs`;
- if `rsynapse-shell` survives `SIGKILL`, treat it as a kernel/FUSE wait and
  collect kernel hung-task logs before rebooting.
