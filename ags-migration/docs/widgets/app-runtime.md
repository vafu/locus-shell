# App Runtime

## AGS Sources Reviewed

- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/commands.ts`
- `/home/v47/.config/ags/style/style.ts`
- `/home/v47/.config/ags/style/theming.ts`
- `/home/v47/.config/ags/services/pomodoro.ts`
- `/home/v47/.config/ags/services/requests.ts`
- `/home/v47/.config/ags/services/hints.ts`
- `/home/v47/.config/ags/commons/index.ts`
- `/home/v47/.config/ags/commons/rx.ts`
- `/home/v47/.config/ags/rxbinding/index.ts`
- `/home/v47/.config/ags/rxbinding/util.ts`
- `/home/v47/.config/ags/scripts/*`
- `/home/v47/.config/ags/systemd/*`
- `/home/v47/.config/ags/triggerhappy/*`

## Runtime Role

The AGS app runtime is the process owner for cross-widget setup. It starts the
GTK application, prepares global theme state, installs the AGS request handler,
sets up command bindings, creates monitor-bound and active-monitor surfaces,
and starts process-wide side effects such as Pomodoro-driven DND.

This is not one visual widget. It is the shared runtime surface that lets the
bar, OSD, agent approval overlay, hints mode, styles, and external scripts
coordinate inside one AGS process.

## Startup Sequence

`app.ts` starts AGS with compiled CSS and `handleRequest` as the request
handler. During `main()` it:

- Initializes Libadwaita.
- Calls `prepareTheme()`.
- Calls `bindCommands()`, which is currently empty and has no migration weight
  until concrete commands are added.
- Starts Pomodoro side effects.
- Starts agent approval auto-open behavior.
- Creates one `Bar` window per monitor from `monitors$`.
- Creates single instances of `OSD` and `AgentApprovalOverlay` bound to the
  first emitted `activeMonitor$`.
- The source also constructs `Rsynapse`, but that launcher surface is confirmed
  unused and excluded from the Rust port.

The runtime treats bars differently from overlays. Bars are per-monitor and
track monitor add/remove diffs. OSD and agent approvals are singleton surfaces
whose monitor input follows the active monitor stream.

## Monitor And Window Lifecycle

`setupForMonitor()` keeps a map from `Gdk.Monitor` to `Gtk.Window`. It compares
successive monitor arrays using `commons/rx.ts` `diffs()`:

- Removed monitors destroy their corresponding window and are removed from the
  map.
- Added monitors create a new widget root and store the resulting GTK window.
- Unchanged monitors are left alone.

This lifecycle only covers per-monitor surfaces. Singleton overlays are created
once after the first active monitor is available and then receive an AGS binding
for subsequent active monitor changes.

## Request Handling

`services/requests.ts` implements a small in-process command bus for `ags
request` calls:

- `handleRequest(argv, handler)` parses the first CLI argument as `command`.
- Remaining arguments are interpreted as key/value pairs.
- Values are parsed as booleans, exact numeric strings, or strings.
- Requests are published to an RxJS `Subject`.
- `requestsFor(...commands)` filters matching commands and returns a handler
  object.
- Handlers reply by calling `handler({ status: 'ok' })`; the original AGS
  request receives the first matching response status.

The active commands visible in reviewed files are:

- `scheme-toggle`: toggles GNOME `org.gnome.desktop.interface color-scheme`.
- `hints`: optionally sets Super-key hints active state from an `active`
  boolean.

`commands.ts` exports `bindCommands()` but does not bind any commands today.

## Hints Runtime

`services/hints.ts` owns a process-local boolean `BehaviorSubject` exposed as
`hintsMode$` and helper methods `show`, `hide`, and `setActive`.

The Super-key state is fed externally by triggerhappy:

- `systemd/ags-super-hints-triggerhappy.user.service` starts `thd` for all
  `/dev/input/event*` devices.
- `triggerhappy/super-hints.conf` calls `scripts/super-hints-trigger` on left
  and right Meta key press/release.
- `scripts/super-hints-trigger` stores left/right key state under
  `$XDG_RUNTIME_DIR/ags-super-hints`, uses `flock`, and emits `ags request
  hints active true|false` only when aggregate active state changes.

The runtime does not read input devices directly; the external service converts
key events into AGS requests.

## Theme And CSS Handling

`style/style.ts` compiles SCSS before app startup and returns the generated CSS
path to AGS.

The compilation pipeline:

- Reads `style/gtk_colors.css` and `style/dyn.css`.
- Extracts GTK `@define-color` entries and generates SCSS variables that point
  back to those GTK color names.
- Finds all `style/**/*.scss` files using `scripts/findstyles.sh`.
- Copies each SCSS file into `/tmp/style/`.
- Writes an aggregate `/tmp/tmp.scss`.
- Runs `sassc /tmp/tmp.scss /tmp/compiled.css`.
- Writes `/tmp/main.css` as GTK colors plus compiled SCSS.

`AstalIO.monitor_file('./style', ...)` watches the style directory. On changed
non-TypeScript files, it recompiles CSS, resets AGS CSS, and reapplies
`/tmp/main.css`.

`style/theming.ts` owns desktop theme side effects:

- `prepareGtk()` syncs the current GNOME color scheme into the GTK theme name
  and `~/.config/niri/theme.kdl` symlink.
- The `scheme-toggle` request flips `prefer-light` and `prefer-dark`.
- GNOME `changed::color-scheme` updates GTK theme and niri theme symlink.
- GNOME `changed::accent-color` runs `scripts/sync_accent.sh`.
- `prepareIcons()` sets the GTK icon theme to `Material`.
- `preparePomodoro()` writes `style/dyn.css` with a dynamic `@define-color bg`
  based on Pomodoro progress.

Pomodoro background color moves from green to yellow to red during `pomodoro`
state and resets to `@theme_bg_color` otherwise.

## Pomodoro And DND Side Effects

`services/pomodoro.ts` wraps the session bus service
`org.gnome.Pomodoro` at `/org/gnome/Pomodoro`.

It exposes:

- State stream with `state`, `elapsed`, `duration`, and `isPaused`.
- Methods `start`, `stop`, `pause`, `skip`, and `toggle`.
- `toggle()` starts when paused and pauses when running.

`app.ts` subscribes to Pomodoro state:

- Entering `pomodoro` runs `scripts/dnd.sh on`.
- Entering `short-break`, `long-break`, or `none` runs `scripts/dnd.sh off`.
- Once a break reaches at least 50% elapsed, it runs
  `scripts/dnd.sh request break_ends`.

`scripts/dnd.sh` requires `AUTOREMOTE_URL`, calls `swaync-client --dnd-on` or
`--dnd-off`, and sends AutoRemote messages `dnd_on`, `dnd_off`, or a custom
request message.

## Agent Approval Auto-Open

`app.ts` watches the selected workspace, finds window sources for that
workspace, follows `window-agent-session` targets, and compares them with
agent sessions that require attention and have a pending prompt.

When a pending session is linked to the selected workspace and the prompt key
has not already been opened, the runtime calls `approvalsUi.showFor(sessionId)`.
This is cross-widget behavior: agent state, Locus workspace/window relations,
and the approval UI are coordinated outside the approval overlay component.

## Shared Reactive Helpers

`commons/rx.ts` provides small RxJS operators:

- `onErrorEmpty()`
- `logNext()`
- `withPrevious(initial)`
- `diffs()`

`rxbinding/index.ts` bridges RxJS, GNIM accessors, and GTK widget lifecycle:

- Subscribe until widget destroy.
- Convert GObject properties and GNIM bindings into observables.
- Convert observables into accessors with initial values.
- Bind object properties by name.
- Follow chained GObject property streams.

`rxbinding/util.ts` includes file monitors, file contents as streams, JSON
subprocess output streams, and periodic command execution.

These helpers are AGS implementation support. Their durable behavior is
lifecycle-bound subscriptions, latest-value bindings, file/process streams,
and simple derived streams.

## Script Inventory

- `scripts/dnd.sh`: swaync DND plus AutoRemote messages.
- `scripts/sync_accent.sh`: writes an accent color CSS file and forces GTK
  theme reload through gsettings.
- `scripts/findstyles.sh`: lists SCSS files under `style/`.
- `scripts/cpu.sh`, `scripts/ram.sh`, `scripts/sysstats.sh`: poll system stats
  through shell commands.
- `scripts/suspend.sh`: runs `systemctl suspend`.
- `scripts/ble_battery`: Python BlueZ GATT battery monitor that prints JSON
  updates.
- `scripts/super-hints-trigger`: stateful triggerhappy bridge to `ags request
  hints active`.

## Visual And Interaction Notes

App runtime has no direct visual design of its own. Its user-visible effects
are:

- Correct creation and teardown of per-monitor bar surfaces.
- Overlay placement following the active monitor.
- Immediate CSS refresh during style changes.
- Desktop theme, icon theme, and niri theme changes applying consistently.
- Pomodoro state changing shell background color and notification DND.
- Super-key press/release showing and hiding hints quickly.
- Agent approval UI opening when the focused workspace contains a pending
  session.

## Boundaries

The AGS runtime mixes process lifecycle, style compilation, desktop settings,
requests, input-device bridges, DND, and widget creation in one application.
For migration, this file documents observed surface/runtime responsibility only.
It should not imply that `shell/core` owns bar, OSD, approval,
Pomodoro, DND, triggerhappy, or command policy.
