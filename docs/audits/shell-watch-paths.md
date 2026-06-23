# Shell Watch/Path/Open-Files Audit

Date: 2026-06-22

Scope: shell-side locusfs source usage in `rsynapse-shell`, `shell/core/src/source`, `LocusPath`
path composition, observable subscription lifecycle, and the log symptoms around
`path escapes mount root`, `Too many open files`, and an unkillable
`rsynapse-shell`.

This audit is read-only. No code changes were made for it.

## Symptoms

### Previous boot hang on 2026-06-22

- `17:15:38`: `rsynapse-shell` logged a relation watch failure:
  `[shell-core/source/relation] /run/user/1000/locusfs/window/5/../../app-instance/codex_%2F3593-5722/agent-session: open relation watch ... failed: path escapes mount root`.
- `17:16:48`: systemd started stopping `rsynapse-shell-dev.service`.
- `17:18:18`: stop timed out and systemd sent `SIGKILL`.
- `17:19:48`: systemd reported `Processes still around after SIGKILL`.
- `17:21:18`: final stop phase also timed out and sent `SIGKILL` again.
- `17:22:48`: unit entered failed mode and still had
  `Unit process 696965 (rsynapse-shell) remains running after unit stopped`.

Kernel logs explain why `SIGKILL` did not help:

- `17:18:46`, `17:20:49`, `17:22:52`: `locusfs` task
  `tokio-rt-worker` was blocked in D-state in
  `fuse_reverse_inval_inode -> fuse_dev_write`.
- `17:20:49`, `17:22:52`: `rsynapse-shell` task `tokio-rt-worker` was blocked
  in D-state in `fuse_readlink_folio -> fuse_symlink_read_folio`.

Interpretation: this was not an ordinary GTK/UI hang. A shell worker was stuck
inside a FUSE symlink read while a `locusfs` worker was stuck writing FUSE
invalidation. A D-state task is not killable until the kernel request returns.

### Locusfs instability in the same boot

`coredumpctl list locusfs` showed many `locusfs` crashes on 2026-06-22, mostly
`SIGSEGV`, plus several `SIGABRT`:

- `12:45:01`, `12:49:49`, `12:50:15`, `12:50:18`
- `12:55:23` (`SIGABRT`)
- `13:02:16` (`SIGABRT`)
- `14:07:15`, `14:07:20`, `14:09:32`, `14:09:37`, `14:10:33`, `14:11:59`,
  `14:14:34`, `14:52:15`, `14:54:45`, `14:56:46`, `15:01:24`, `16:09:39`,
  `16:10:49`
- `16:36:42` (`SIGABRT`)

Focused `locusfs.service` logs:

- `14:10:33`, `14:11:59`, `14:14:34`: service stop triggered
  `locusfs-pipewire: pactl subscribe ended`, then `locusfs` dumped core with
  `status=11/SEGV`.
- `14:11:10` and `14:12:46`: `locusfs-statusnotifier`/zbus thread panicked:
  `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.
- `17:22:39` through reboot at `17:24:10`: `locusfs-niri` and
  `locusfs-pipewire` repeatedly retried after the session was already tearing
  down:
  `failed to reconnect event stream: No such file or directory`,
  `failed to read PipeWire snapshot: pactl -f json info failed: Connection failure: Connection terminated`,
  and `pactl subscribe ended`.

Interpretation: shell-side watch pressure is real, but the unrecoverable hang
also needs a locusfs-side fix. `locusfs` must not let plugin reconnect loops or
FUSE invalidation block indefinitely during shutdown/session teardown.

### Open file exhaustion

User-observed shell log:

```text
[shell-core/source/property] /run/user/1000/locusfs/dbus-service/networkmanager/object/AccessPoint%2F8/DeviceType: open watch ... failed: Too many open files (os error 24)
```

User-observed `lsof | wc -l` grew rapidly after app restarts:

- `171788`
- `521124`
- `542699`

Interpretation: source subscriptions are opening far too many independent
watch/read descriptors and/or locusfs is retaining stale handles after clients
restart. The shell has a direct multiplier because source primitives are cold
observables without descriptor-keyed sharing.

## Code Findings

### 1. `LocusPath` does not normalize lexical `..`

Location: `shell/core/src/locus_path/mod.rs:13-63`

`LocusPath::new` stores the incoming `PathBuf` unchanged, and
`LocusPath::child` is plain `PathBuf::join`:

- `LocusPath::new`: line 14
- `LocusPath::child`: line 30
- `prop`/`rel`: lines 40-45

This preserves paths like:

```text
/run/user/1000/locusfs/window/5/../../app-instance/.../agent-session
```

Those paths are then passed directly into `as_relation`, `as_property`, or
`as_children`. `locusfs-watch` rejects at least one of them as escaping the mount
root, matching the `17:15:38` log.

Likely root cause: relation reads can return symlink targets containing relative
segments, and shell code composes additional children on top of that target
without normalizing within the mount root.

### 2. `relation` normalizes watch events differently from initial read

Location: `shell/core/src/source/relation.rs:104-134`

On watch event `Set(Path)`, code uses `watch_value_path(mount_root, value)`:

- `WatchValue::Path` handling: lines 110-113
- `watch_value_path`: lines 120-127

But initial read and change fallback call `read_relation`:

- initial read: line 69
- `Change(_)` fallback: line 115
- `read_relation`: lines 129-134

`read_relation` wraps `locusfs_watch::read_link(path)` as `LocusPath::new(value)`
with no mount-root normalization. This means the same relation can emit
different path shapes depending on whether the value came from the initial
read/fallback or from a structured watch `set` event.

Concrete risk:

- Initial relation target may remain `/run/user/1000/locusfs/window/5/../../app-instance/...`.
- Later child/property composition can produce path escape errors.
- `distinct_until_changed` may see normalized and non-normalized forms as
  different even if they refer to the same node.

### 3. Source primitives are cold and explicitly not shared

Location: `shell/core/src/source/mod.rs:196-241`

The public source API has TODOs for descriptor-keyed sharing:

- `watch`: lines 196-200
- `property`: lines 204-214
- `relation`: lines 217-224
- `node`: lines 227-231

Each call to `observe_prop`, `observe_rel`, `as_children`, or `as_node` builds a
new cold observable. Each subscription creates its own stream and opens its own
watch.

This is the most likely shell-side contributor to `Too many open files`.

### 4. Every stream subscription spawns a Tokio task and aborts on unsubscribe

Location: `shell/core/src/source/support.rs:18-111`

- `from_stream_result` wraps a stream through `Shared::<()>::lift(...)`.
- `CoreObservable::subscribe` spawns `drive_stream` with `tokio::spawn`.
- `AbortOnUnsubscribe::unsubscribe` calls `JoinHandle::abort()`.

This gives a disposal hook, but it is abrupt and does not wait for the FUSE
operation to return. If the task is inside `read_link`, `read_to_string`,
`Watch::open`, or `wait_raw_event`, cancellation can be delayed by the underlying
kernel/FUSE operation. In the observed hang, a shell worker was blocked in
`fuse_readlink_folio`, so Rust-level abort could not make it disappear.

### 5. `open_target_or_parent` may subscribe to broad ancestors

Location: `shell/core/src/source/support.rs:151-208`

If `Watch::open(path)` gets a missing-path error, the code finds the nearest
existing parent using `path.exists()` and opens a watch on that ancestor.

This is useful for missing properties, but it can multiply broad watches and
reopen churn:

- every missing property can open a parent watch;
- for path-shaped bugs, the nearest ancestor may be unrelated to the intended
  logical node;
- `path.exists()` itself hits FUSE repeatedly during source startup.

### 6. Consumer source fan-out is high

The consumer follows the observable-only rule, but several providers combine
large collections by opening per-child/per-property observables.

High-fanout examples:

- `rsynapse-shell/src/widgets/bar/network/mod.rs:82-160`
  - one `as_children` for NetworkManager devices and another for access points;
  - per device: `DeviceType`, `State`, `Interface`, `ActiveAccessPoint`;
  - per access point: `Ssid`, `Strength`.
- `rsynapse-shell/src/widgets/bar/bluetooth/source.rs:48-120`
  - BlueZ and UPower children;
  - per BlueZ object: `Powered`, `Discovering`, `Connected`, `Connecting`,
    `Name`, `Address`, `Class`, `Appearance`, `BatteryPercentage`;
  - per UPower object: `NativePath`, `Percentage`.
- `rsynapse-shell/src/widgets/bar/audio/source.rs:23-100`
  - `audio_status` and `audio_routes` both independently observe default sink
    and the complete sink list;
  - per sink: up to 9 properties.
- `rsynapse-shell/src/widgets/bar/project_label/source/agent_aggregate.rs:48-187`
  - for every workspace/project label, watches global `agent-session`,
    `app-instance`, and `window` collections plus relations/properties under
    every child.
- `rsynapse-shell/src/widgets/bar/systray/source.rs:54-150`
  - systray items are relation targets;
  - each item watches node state and seven properties;
  - each menu item watches five properties.

This code shape is reasonable only if `shell_core::source` provides sharing and
replay by descriptor. Without sharing, the number of open FDs scales with:

```text
subscribers * collection_size * watched_fields_per_item
```

and restarts can amplify the problem if FUSE handles linger.

### 7. Macro subscription ownership is mostly present, but nested source handling needs review

Locations:

- `shell/macros/src/locus_bindings/component.rs:206-223`
- `shell/macros/src/locus_bindings/expand.rs:672-707`
- `shell/macros/src/locus_bindings/expand.rs:750-798`
- `shell/core/src/model.rs:21-32`

Top-level generated subscriptions are stored in model runtime through
`set_subscriptions`, and use `unsubscribe_when_dropped`.

However, generated nested source model code creates a `context_subscriptions`
vector inside `start_source_model`, clears/replaces it inside the subscription
closure, but only pushes the outer subscription into the returned
`subscriptions` vector. This deserves direct verification with expanded code:

- if the closure retains the inner subscription guards, it may keep dynamic
  subscriptions alive until the next context update, but the ownership is not
  obvious;
- if the closure drops them too early, nested source contexts can be dead;
- either way, this is a fragile place for watch churn and should be verified
  with an explicit unit/expanded-code test.

## Root-Cause Hypotheses

### A. Path normalization bug causes invalid relation targets

Confidence: high.

Evidence:

- `17:15:38` log has a concrete `../../` path rejected as escaping mount root.
- `LocusPath::new` and `child` preserve `..`.
- `relation::read_relation` does not normalize using the watch mount root, while
  `WatchValue::Path` events do partial mount-root joining.

Fix direction:

- Normalize relation targets at the source boundary, not in widget code.
- Make relation initial read and watch-event paths use the same resolver.
- Lexically normalize inside the known locusfs mount root, without following
  symlinks through `std::fs::canonicalize`.
- Reject targets that leave the mount root before emitting a `LocusPath`.

### B. Missing descriptor-keyed sharing causes FD explosion

Confidence: high.

Evidence:

- Public source API TODOs already identify missing share/replay.
- User saw `Too many open files`.
- Consumer source graph has high fan-out and repeated sources for the same
  paths.

Fix direction:

- Add descriptor-keyed shared latest observables in `shell_core::source`.
- Keys should include primitive kind plus normalized path, and for typed
  properties also the target Rust type if necessary.
- Sharing should be ref-counted: first subscriber opens the watch; last
  subscriber closes it; new subscribers receive replayed latest value.
- Widget code should not add local caches around source helpers.

### C. Abrupt abort is not enough for blocked FUSE operations

Confidence: medium-high.

Evidence:

- `AbortOnUnsubscribe` calls `JoinHandle::abort()`.
- Kernel shows shell task blocked in `fuse_readlink_folio`, which Rust abort
  cannot unwind until the syscall returns.

Fix direction:

- Keep cancellation cooperative at the observable layer, but treat FUSE blocking
  as a locusfs/client responsibility too.
- Make `locusfs-watch`/locusfs ensure watch/read operations unblock promptly on
  daemon teardown and do not wait behind invalidation deadlocks.
- Add timeout only as diagnostics if needed, not as correctness behavior.

### D. Locusfs plugin shutdown and FUSE invalidation can deadlock clients

Confidence: high for locusfs-side involvement; shell audit only.

Evidence:

- `locusfs` worker blocked in `fuse_reverse_inval_inode -> fuse_dev_write`.
- shell worker blocked in `fuse_readlink_folio`.
- `locusfs` plugins were retrying during session teardown.
- repeated `locusfs` core dumps and zbus runtime panic happened earlier in the
  same boot.

Fix direction:

- In locusfs, stop plugin reconnect loops on cancellation/shutdown.
- Fix statusnotifier zbus executor to run inside a Tokio runtime.
- Audit FUSE invalidation path for blocking writes while clients are blocked on
  symlink reads.
- Add stale-handle cleanup/forced teardown behavior in locusfs if clients die or
  restart.

### E. Nested source model subscription ownership may create churn or leaks

Confidence: medium.

Evidence:

- Generated code around dynamic context subscriptions is non-obvious and should
  be verified with expanded output.
- The user observed list churn and watch close/reopen patterns earlier.

Fix direction:

- Generate nested dynamic source context as an explicit `switch_map`-like
  observable where possible, so Rx owns the dynamic subscription lifecycle.
- If macro-owned nested subscriptions remain, add tests proving old inner
  subscriptions are dropped exactly once when context changes and when the
  component drops.

## Proposed Fixes

### Shell-core source

1. Introduce a relation-target resolver used by both initial read and watch
   events.
   - Input: watch path, mount root, relation value.
   - Output: normalized `LocusPath` inside mount root or a logged source error.
   - Do lexical normalization only; do not call `canonicalize` on locusfs paths.

2. Normalize `LocusPath` construction for shell-composed paths.
   - `child`, `prop`, and `rel` should not leave literal `..` segments in paths
     that are known to be inside the locusfs mount.
   - Add tests for:
     - `/run/user/1000/locusfs/window/5/../../app-instance/x`
     - relation targets that are absolute logical paths such as `/app-instance/x`
     - relation targets that are mount-absolute paths
     - attempted escape above mount root.

3. Add descriptor-keyed shared latest observables.
   - Start with `property`, `relation`, `children`, and `node`.
   - Keep public API unchanged.
   - Use normalized path in the key.
   - Preserve current `distinct_until_changed` semantics.
   - Add counters/logs while validating: open watch count, close watch count,
     active descriptors by key.

4. Revisit `open_target_or_parent`.
   - Ensure missing child/property watches use the narrowest stable parent.
   - Log whether the opened watch is target or ancestor while validating.
   - Avoid repeated `path.exists()` on already-known bad/escaping paths.

5. Make source error logs include the source key and phase.
   - Current logs identify primitive and path; add `open`, `initial_read`,
     `event_read`, `fallback_read`, and `normalization`.

### Shell macros / lifecycle

1. Inspect expanded generated code for nested source models.
2. Add a focused test that a dynamic nested source context:
   - subscribes to the new context;
   - unsubscribes from the old context once;
   - unsubscribes from the active context when the component/model drops.
3. Prefer Rx `switch_map` for dynamic dependencies where possible, so generated
   code does not hand-roll context subscription replacement.

### Rsynapse-shell consumer sources

1. Keep using `shell_core::source` observables only; do not introduce local
   watcher/read APIs.
2. After core sharing exists, re-check high-fanout providers:
   - audio: share sink snapshots between `audio_status` and `audio_routes`;
   - project label: avoid recomputing global agent/window/app aggregates for
     every workspace when one shared aggregate source would do;
   - systray/dbusmenu: ensure stale nodes are filtered by node state and source
     sharing prevents duplicate property watches.
3. Do not fix FD pressure with debounce/throttle/sleep.

### Locusfs-side follow-up

This audit focused on shell-side code, but the hang cannot be fully fixed only
in the shell:

1. Fix `locusfs-statusnotifier` zbus executor runtime panic.
2. Stop pipewire/niri reconnect loops on service shutdown/session teardown.
3. Audit FUSE invalidation path around `fuse_reverse_inval_inode`.
4. Add stale watch/file-handle guardrails so restarted clients do not leave
   unbounded resources.
5. Ensure blocked shell reads return promptly when locusfs is stopping or the
   mount is being torn down.

## Validation Checklist

### Before fix

- Capture active shell watch count:
  `lsof -p $(pidof rsynapse-shell) | rg '/run/user/1000/locusfs|/watch' | wc -l`.
- Capture locusfs handle count if exposed by logs/metrics.
- Reproduce opening/closing bar popovers and restarting `rsynapse-shell`.
- Confirm whether watch count returns to baseline after closing UI or stopping
  the shell.

### Path normalization

- Run unit tests for `LocusPath` normalization and relation target resolution.
- Start shell with agent windows that use `app-instance -> agent-session`.
- Confirm no logs contain:
  - `path escapes mount root`
  - paths under locusfs with literal `/../`
  - relation targets whose normalized form differs between initial read and
    watch `set` event.

### FD sharing

- Start `rsynapse-shell`.
- Open and close audio, bluetooth, systray, and workspace-heavy views.
- Restart shell several times.
- Verify:
  - `lsof | wc -l` does not monotonically increase;
  - shell process watch FDs stay bounded;
  - no `Too many open files (os error 24)` logs;
  - descriptor-key logs show one upstream watch per shared source key.

### Watch lifecycle

- Switch workspaces rapidly.
- Add/remove windows.
- Add/remove systray items.
- Connect/disconnect bluetooth devices.
- Confirm:
  - old dynamic child subscriptions are dropped;
  - selected workspace/window lists update without one-by-one remove/add churn
    caused by source recreation;
  - no pending watch handles remain after component removal.

### Shutdown/hang

- Stop `rsynapse-shell-dev.service`; it should exit before systemd timeout.
- Stop `locusfs.service`; it should not dump core.
- During stop, kernel log should not show blocked tasks in:
  - `fuse_reverse_inval_inode`
  - `fuse_dev_write`
  - `fuse_readlink_folio`
- During session teardown, locusfs plugins should stop instead of retrying
  `pactl`/niri forever.

## Immediate Risk Notes

- The `path escapes mount root` bug is shell-side and should be fixed before
  relying on agent/session relation sources.
- The `Too many open files` symptom is likely shell-side fan-out plus missing
  source sharing, with possible locusfs stale-handle amplification.
- The unkillable process is kernel/FUSE-level; shell cleanup alone cannot
  recover a task stuck in `fuse_readlink_folio`.
- The `locusfs-statusnotifier` zbus panic and repeated locusfs core dumps are
  separate blockers for a stable bar, even if shell path/sharing issues are
  fixed.
