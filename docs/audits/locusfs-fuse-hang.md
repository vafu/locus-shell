# locusfs FUSE hang audit

Audit date: 2026-06-23.
Observed failure boot: 2026-06-22, user boot `-1`.

Scope: log and code audit only. No code was changed.

## Summary

The 2026-06-22 hang was not a normal rsynapse-shell UI freeze. Kernel logs show
a FUSE deadlock-style failure:

- locusfs pid 642141 had a Tokio worker stuck in kernel D-state while writing a
  FUSE notify/invalidation response.
- rsynapse-shell pid 696965 had a Tokio worker stuck in kernel D-state while
  resolving a FUSE symlink with `readlink`.
- systemd could not kill rsynapse-shell with SIGKILL because the task was stuck
  in uninterruptible kernel sleep.

There are also independent locusfs shutdown/runtime problems:

- repeated locusfs coredumps during stop/restart,
- statusnotifier zbus runtime panics,
- pipewire and niri reconnect loops continuing during session teardown,
- shell paths containing `../../` that can escape the locusfs mount root.

The strongest root-cause hypothesis is a race/cycle between locusfs kernel
invalidation of relation/symlink inodes and clients resolving relation symlinks,
made easier to trigger by plugin reconnect/shutdown churn.

## Evidence

### Repeated locusfs coredumps

`coredumpctl --no-pager list locusfs` shows repeated crashes on 2026-06-22:

- 12:45:01 SIGSEGV
- 12:49:49 SIGSEGV
- 12:50:15 SIGSEGV
- 12:50:18 SIGSEGV
- 12:55:23 SIGABRT
- 13:02:16 SIGABRT
- 14:07:15 SIGSEGV
- 14:07:20 SIGSEGV
- 14:09:32 SIGSEGV
- 14:09:37 SIGSEGV
- 14:10:33 SIGSEGV
- 14:11:59 SIGSEGV
- 14:14:34 SIGSEGV
- 14:52:15 SIGSEGV
- 14:54:45 SIGSEGV
- 14:56:46 SIGSEGV
- 15:01:24 SIGSEGV
- 16:09:39 SIGSEGV
- 16:10:49 SIGSEGV
- 16:36:42 SIGABRT

Representative log lines:

- 14:11:59: `locusfs.service: Main process exited, code=dumped, status=11/SEGV`
- 14:11:59: `fusermount3: entry for /run/user/1000/locusfs not found in /etc/mtab`
- 14:14:34: `locusfs-pipewire: pactl subscribe ended`
- 14:14:34: `locusfs.service: Main process exited, code=dumped, status=11/SEGV`

The visible coredump stack for 14:14:34 is in Tokio runtime shutdown:

- `tokio::runtime::time::wheel::Wheel::process_expiration`
- `tokio::runtime::time::Driver::shutdown`
- `tokio::runtime::blocking::pool::BlockingPool::shutdown`
- `drop_in_place<tokio::runtime::runtime::Runtime>`
- `locusfs::main`

This points at task/runtime shutdown pathology, not at a clean unmount.

### statusnotifier zbus runtime panic

After a restart:

- 14:12:31: `locusfs-statusnotifier: zbus executor task started`
- 14:12:46: thread panicked in `zbus-5.16.0/src/abstractions/executor.rs:63:27`
- 14:12:46: `there is no reactor running, must be called from the context of a Tokio 1.x runtime`

The same panic repeats at 14:15:12 after another restart.

At 16:36:42 the coredump stack includes
`liblocusfs_plugin_statusnotifier.so` running its own current-thread Tokio
runtime inside `StatusNotifierRuntime::start`.

### Actual hang sequence

rsynapse-shell service log:

- 17:11:22: rsynapse-shell pid 696965 starts.
- 17:15:38: shell logs an invalid locusfs relation path:
  `open relation watch ... /run/user/1000/locusfs/window/5/../../app-instance/.../agent-session failed: path escapes mount root`
- 17:16:48: systemd starts stopping rsynapse-shell.
- 17:18:18: stop-sigterm times out; systemd sends SIGKILL.
- 17:19:48: `Processes still around after SIGKILL. Ignoring.`
- 17:21:05: SIGKILL sent again on client request.
- 17:22:48: `Processes still around after final SIGKILL. Entering failed mode.`
- 17:22:48: `Unit process 696965 (rsynapse-shell) remains running after unit stopped.`

Kernel log:

- 17:18:46: locusfs pid 642141 / thread 642184:
  `INFO: task tokio-rt-worker:642184 blocked for more than 122 seconds`
- 17:20:49: same locusfs task blocked for more than 245 seconds.
- 17:22:52: same locusfs task blocked for more than 368 seconds.

The locusfs kernel stack is:

```text
invalidate_inode_pages2_range
fuse_reverse_inval_inode
fuse_dev_do_write
fuse_dev_write
vfs_writev
do_writev
```

That means locusfs was blocked while issuing a FUSE reverse invalidation / notify
write to the kernel.

Kernel log for rsynapse-shell:

- 17:20:49: rsynapse-shell pid 696965 / thread 702129:
  `INFO: task tokio-rt-worker:702129 blocked for more than 122 seconds`
- 17:22:52: same task blocked for more than 245 seconds.

The rsynapse-shell kernel stack is:

```text
__fuse_simple_request
fuse_readlink_folio
fuse_symlink_read_folio
do_read_cache_folio
__page_get_link
pick_link
step_into_slowpath
link_path_walk
```

That means the shell was blocked resolving a locusfs symlink.

### Plugin teardown churn near reboot

From 17:22:39 until the boot ends at 17:24:10, locusfs logs repeated teardown
errors:

- `locusfs-niri: failed to read event stream: EOF while parsing a value at line 1 column 0`
- `locusfs-pipewire: pactl subscribe ended`
- `locusfs-niri: failed to reconnect event stream: I/O failed: No such file or directory (os error 2)`
- `locusfs-pipewire: failed to read PipeWire snapshot: I/O failed: pactl -f json info failed: Connection failure: Connection terminated`

The niri error repeats about once per second. The pipewire snapshot error repeats
about every two seconds. This is consistent with plugins still running reconnect
loops while the graphical/session services are already going away.

## Code findings

### FUSE invalidation path

Relevant files:

- `../locusfs/fuse/src/mount.rs`
- `../locusfs/fuse/src/invalidation.rs`
- `../locusfs/fuse/src/fs/watch.rs`

`mount.rs` starts a global graph change invalidator after mounting:

- `mount.rs:51-70`: creates `WatchRegistry`, `SharedKernelNotify`, and
  `spawn_change_invalidator`.
- `mount.rs:39-47`: `FuseMount::unmount` first calls
  `self.change_worker.shutdown()`, then awaits `session.unmount()`.

`invalidation.rs` owns the FUSE notify path:

- `invalidation.rs:44-69`: graph change loop receives changes and calls
  `invalidate_change(...).await`.
- `invalidation.rs:20-27`: shutdown sends a oneshot and immediately aborts the
  task; it does not await task completion.
- `invalidation.rs:282-307`: relation changes call
  `invalidate_matching_relation_inodes(...)` and then
  `retarget_relation_watchers(...)`.
- `invalidation.rs:337-352`, `488-501`, `505-525`: known inode invalidation
  calls `notifier.invalid_inode(ino, 0, 0).await`.
- `invalidation.rs:457-463`: watch poll wakeups call
  `notifier.wakeup(handle).await`.

The kernel stack maps directly to these `Notify::invalid_inode` calls. The
worker is blocked in the kernel while doing the notify write.

`fs/watch.rs` relation retargeting:

- `watch.rs:478-485`: dependent watchers are looked up by relation dependency.
- `watch.rs:488-526`: retarget result detaches/reattaches watch subjects and
  queues state/change events.
- `watch.rs:201-213`: normal watch configuration clears pending reads/events.
- `watch.rs:324-328`: release detaches watch and removes the open file state.

Retargeting itself may be correct, but the current implementation combines
kernel inode invalidation and watch event retargeting in the same graph-change
handling path. If kernel invalidation blocks, relation watch retarget delivery
can be delayed or never happen.

### statusnotifier runtime

Relevant files:

- `../locusfs/plugins/statusnotifier/src/lib.rs`
- `../locusfs/plugins/statusnotifier/src/runtime.rs`

`runtime.rs:36-45` starts statusnotifier inside `runtime.spawn_blocking`, builds
a separate current-thread Tokio runtime, and then calls `runtime.block_on(...)`.

`runtime.rs:138-149` is an infinite retry loop with `sleep_retry()`.

`runtime.rs:162-176` builds the zbus session connection and serves the watcher
object.

`runtime.rs:195-232` has a `tokio::select!` over commands and DBus owner
changes, but no shutdown branch.

The observed zbus panic says a zbus executor task ran without a Tokio reactor.
The separate runtime inside a blocking task is the first place to inspect. The
statusnotifier plugin should either run on the host Tokio runtime or own a
dedicated runtime whose zbus executor and all spawned item watchers are
unambiguously entered and shut down.

### pipewire runtime

Relevant files:

- `../locusfs/plugins/pipewire/src/lib.rs`
- `../locusfs/plugins/pipewire/src/runtime.rs`

`lib.rs:42-48` aborts and awaits the pipewire task during plugin shutdown.

`runtime.rs:37-83` is an infinite outer loop:

- spawn `pactl subscribe`,
- read lines,
- on end/error kill the child,
- sleep two seconds,
- refresh snapshot,
- repeat.

`runtime.rs:91-114` emits graph changes after each snapshot.

There is no explicit cancellation token checked inside `lines.next_line()`,
`sleep_retry()`, or `refresh_and_publish()`. Aborting the task should usually
work, but logs show this runtime still produced repeated retry errors while the
session was shutting down. It needs cooperative cancellation and "do not publish
after shutdown starts" semantics.

### niri runtime

Relevant files:

- `../locusfs/plugins/niri/src/lib.rs`
- `../locusfs/plugins/niri/src/ipc.rs`

`lib.rs:45-50` aborts and awaits the niri event stream task.

`ipc.rs:48-63` is an infinite reconnect loop. After `read_event_stream` exits,
it sleeps and repeatedly attempts `connect_event_stream()`.

`ipc.rs:82-103` emits graph changes for every niri event and breaks on read
error.

There is no shutdown branch in the reconnect loop or read loop. During session
teardown, once the niri socket disappears, the plugin keeps retrying and logging
until the process exits or the task is aborted.

### Plugin host shutdown order

Relevant files:

- `../locusfs/bin/src/main.rs`
- `../locusfs/bin/src/plugin/mod.rs`
- `../locusfs/plugins/api/src/lib.rs`

`main.rs:73-75` does:

```text
wait_for_shutdown()
plugins.shutdown().await
unmount_with_fallback(mount, &mountpoint).await
```

`plugin/mod.rs:53-65` calls plugin shutdown sequentially.

`plugins/api/src/lib.rs:90-92` has a default no-op `PluginHandle::shutdown`.
Most active plugins implement shutdown, but the contract itself does not force
cooperative cancellation, no-late-publish behavior, or join completion before
unmount.

## Root-cause hypotheses

### H1: FUSE reverse invalidation can deadlock with symlink resolution

Strongest hypothesis.

The kernel reports locusfs blocked in `fuse_reverse_inval_inode` and
rsynapse-shell blocked in `fuse_readlink_folio`. That is the exact pair expected
when the daemon is trying to invalidate a symlink/relation inode while a client
is waiting for the daemon to serve symlink resolution.

Potential trigger:

1. Plugin or niri emits relation/property graph changes.
2. `invalidation.rs` invalidates known relation inodes via
   `Notify::invalid_inode`.
3. A shell watch/read path resolves a relation symlink at the same time.
4. The kernel waits for page/link state while the daemon worker is blocked in
   reverse invalidation and another client request waits for the daemon.

### H2: Plugin shutdown churn keeps publishing graph changes while FUSE is unstable

The 17:22 logs show pipewire/niri retry loops continuing during session
teardown. Those loops can emit graph changes or attempt reads while clients and
services are stopping. Even if they are not the root cause, they create extra
invalidation pressure at the worst time.

### H3: statusnotifier runtime is incorrectly hosted

The zbus panic is concrete and repeated. It may not be the direct 17:18 kernel
deadlock trigger, but it is definitely a locusfs runtime bug. The coredump stack
also shows `liblocusfs_plugin_statusnotifier.so` owning a nested current-thread
runtime.

### H4: `../../` relation paths increase bad symlink/watch behavior

The shell logged a path escaping the mount root:

```text
/run/user/1000/locusfs/window/5/../../app-instance/.../agent-session
```

That path should never reach `locusfs-watch` as a watch path. This is likely a
shell-core `LocusPath` normalization bug or a relation-target composition bug.
It is probably not the kernel deadlock by itself, but it increases relation
watch churn and failed symlink resolution.

### H5: abort-only shutdown is not enough for kernel/FUSE operations

`InvalidationWorker::shutdown` aborts without awaiting. Plugin tasks often abort
and await, but their inner loops are not cancellation-aware. Tokio abort cannot
unblock a task already inside an uninterruptible kernel FUSE operation. Shutdown
needs to stop new graph changes before unmount and avoid entering kernel notify
calls once teardown begins.

## Recommended fixes

### 1. Make plugin runtime shutdown cooperative

Add an explicit cancellation token or shutdown receiver to `PluginContext`.
Each plugin runtime should:

- stop reconnect loops when cancellation is requested,
- stop emitting graph changes after cancellation starts,
- select cancellation against long waits such as `lines.next_line()`,
  `sleep_retry()`, DBus signal streams, and niri socket reads,
- kill/drop child processes during cancellation,
- return from the task cleanly so `PluginHandle::shutdown` can await completion.

Start with niri, pipewire, statusnotifier, dbusmenu, and mpris.

### 2. Fix statusnotifier runtime hosting

Do not run zbus watcher logic in an ad hoc `spawn_blocking` current-thread
runtime unless every zbus executor task is guaranteed to live inside that
runtime.

Preferred direction:

- run statusnotifier on the host Tokio runtime like other plugins, or
- make a dedicated runtime a first-class owned object with explicit shutdown,
  and never pass handles across runtime boundaries ambiguously.

The fix is verified only when the `there is no reactor running` panic disappears
across repeated locusfs restarts.

### 3. Harden FUSE invalidation against relation/symlink races

Audit all `Notify::invalid_inode` calls for relation links and relation target
links. Avoid invalidating symlink inodes in a way that can block while clients
are resolving the same link.

Candidate approaches to test:

- Prefer parent entry invalidation for relation directory entries if fuse3
  exposes a safe `invalid_entry` path.
- Do not invalidate relation symlink inodes inline before watch retarget events
  have been queued.
- Decouple graph-change watch notifications from kernel inode invalidation so
  `/watch` clients still receive state even if kernel notify is slow.
- Make invalidation best-effort during shutdown: once teardown begins, skip
  kernel invalidation and only unblock/close watches.
- Add instrumentation around every `invalid_inode` and `wakeup` call with inode,
  entry kind, elapsed time, and graph change kind.

### 4. Make shutdown ordering explicit

Recommended service shutdown order inside locusfs:

1. Mark graph/plugins as shutting down so no new plugin-originated graph changes
   are accepted.
2. Signal plugins to stop and await clean completion.
3. Drain or close graph change receivers.
4. Stop the invalidation worker after outstanding accepted changes are handled,
   or explicitly discard remaining changes with a shutdown log.
5. Clear/disable kernel notifier.
6. Unmount.

The important invariant: no plugin should emit a graph change after FUSE
unmount/invalidation shutdown starts.

### 5. Fix shell path normalization separately

In shell-core, make `LocusPath` relation composition normalize paths within the
mount root before opening watches. `../../` segments should not survive into
watch paths. If a relation target is relative, resolve it to a canonical
mount-local logical path first.

This is a separate fix from locusfs, but it removes a known bad client-side
input from the failure surface.

## Verification after fixes

Run these checks after the locusfs fixes:

- Start `locusfs.service` and `rsynapse-shell-dev.service`; switch workspaces,
  add/remove windows, and open systray/dbusmenu/pipewire widgets.
- `systemctl --user stop rsynapse-shell-dev.service` must complete without
  lingering processes.
- `systemctl --user stop locusfs.service` must complete without coredump.
- `coredumpctl --no-pager list locusfs` must show no new entries.
- `journalctl -k -b` must show no new `INFO: task ... blocked for more than`
  entries involving `locusfs`, `rsynapse-shell`, or `fuse`.
- During session teardown or manual compositor/PipeWire restart, niri and
  pipewire plugins should log one shutdown/cancellation message, not unbounded
  reconnect spam.
- Repeated `systemctl --user restart locusfs.service` should not produce the
  zbus `no reactor running` panic.
- `locusfs --watch /run/user/1000/locusfs/context/selected/workspace` and the
  shell should both continue receiving relation updates after workspace switches.
- `lsof` count should stabilize after restarting the shell several times; watch
  file descriptors should not grow unbounded.

## Immediate mitigation until fixed

Disable `statusnotifier` first if the hang reproduces frequently. It has a
confirmed zbus runtime panic and coredump evidence. If hangs continue, disable
`pipewire` and `niri` plugin reconnect loops temporarily during shutdown testing
to isolate the FUSE invalidation path.
