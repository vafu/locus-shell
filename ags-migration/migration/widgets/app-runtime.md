# App Runtime Migration

## AGS Sources Reviewed

- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/commands.ts`
- `/home/v47/.config/ags/style/style.ts`
- `/home/v47/.config/ags/style/theming.ts`
- `/home/v47/.config/ags/services/pomodoro.ts`
- `/home/v47/.config/ags/services/requests.ts`
- `/home/v47/.config/ags/services/hints.ts`
- `/home/v47/.config/ags/commons/*`
- `/home/v47/.config/ags/rxbinding/*`
- `/home/v47/.config/ags/scripts/*`
- `/home/v47/.config/ags/systemd/*`
- `/home/v47/.config/ags/triggerhappy/*`

## Scope

Port the AGS app runtime as `rsynapse-shell` consumer infrastructure. This
proposal describes process layout, monitor/window lifecycle, request routing,
CSS/theme preparation, Pomodoro/DND side effects, and shared runtime helpers.

It is not a `shell/core` mandate. `shell/core` should continue to expose generic
GTK, Relm4, layer-shell, CSS registration, provider runtime, and window
primitives. `rsynapse-shell` owns product-specific surfaces and policies.

## Proposed Process Layout

Prefer multiple Rust binaries instead of recreating one all-owning AGS process:

- `rsynapse-bar`: per-monitor top bar windows.
- `rsynapse-osd`: active-monitor OSD overlay.
- `rsynapse-agent-approvals`: active-monitor approval overlay plus approval UI
  request handling.
- `rsynapse-runtime`: optional small coordinator for commands, theme side
  effects, hints mode, Pomodoro/DND side effects, and agent approval auto-open.

If startup simplicity matters more than isolation for the first migration
phase, `rsynapse-shell` can start as one binary with internal modules matching
the split above. The target shape should still preserve the process boundaries
from `PROJECT.md`: major widgets can fail independently, and product policy
does not move into `shell/core`.

## Runtime Ownership

`rsynapse-shell` should own:

- Which widget binaries exist and how they are supervised.
- Per-monitor versus singleton surface policy.
- App-specific command names and request payloads.
- Theme and niri side effects.
- Pomodoro to DND/AutoRemote behavior.
- Triggerhappy/systemd integration for Super-key hints.
- Agent approval auto-open policy.
- Shell script replacement or retention decisions.

`shell/core` should only provide:

- Generic app startup.
- Provider task runtime installation.
- CSS registration and optional development file watching.
- Generic layer-shell window creation.
- Generic lifecycle primitives needed by consumers.

## Monitor And Window Lifecycle

The AGS bar lifecycle maps cleanly to a consumer-owned monitor manager:

- Subscribe to monitor topology from GTK/GDK or a provider exposed by
  `rsynapse-shell`.
- Keep a `MonitorId -> WindowHandle` map rather than keying directly by object
  identity.
- On monitor add, create a bar component using `shell_core::window` with bar
  placement policy defined in `rsynapse-bar`.
- On monitor remove, close the matching window and cancel any provider
  subscriptions owned by that window/component.
- Leave unchanged monitors alone unless their geometry/scale/name changes in a
  way that requires surface reconfiguration.

Active-monitor overlays should not be recreated on every focus change. Keep
one process/window per overlay role and update a typed `active_monitor` model
field or provider input. The overlay component owns how it repositions or
reconfigures its layer-shell surface when the active monitor changes.

Candidate missing shell feature:

- [monitor-provider.md](../../missing-shell-features/monitor-provider.md)
- [window-lifecycle-handles.md](../../missing-shell-features/window-lifecycle-handles.md)

## Request And Command Handling

Replace `ags request` with a typed request surface owned by `rsynapse-shell`.
The first migration target can be a small session D-Bus interface because it
matches the rest of the shell's provider direction and avoids a custom CLI
daemon protocol.

Suggested interface shape:

- `io.github.Rsynapse.Shell.Request(command: String, args: Dict<String, Variant>) -> status`
- A thin `rsynapsectl` CLI translates command-line pairs into the D-Bus call.
- Runtime modules register typed handlers for known commands.
- Unknown commands return an error instead of hanging.
- Request handlers reply once and include at least `ok` or `error`.

Initial commands:

- `scheme-toggle`: toggles GNOME light/dark preference.
- `hints active=<bool>`: sets Super-key hints mode.
- Approval commands, if needed by the migrated approval UI.

Keep `bindCommands()` as an `rsynapse-shell` module concept only if concrete
commands appear. The reviewed AGS implementation exports it but does nothing.

Candidate missing shell feature:

- [consumer-request-service.md](../../missing-shell-features/consumer-request-service.md)

## Theme And CSS Handling

Preserve the behavior, not the AGS compilation path:

- Keep visual CSS/SCSS in external files owned by `rsynapse-shell`.
- Build SCSS to CSS as a development/build step where possible.
- In development, use `shell-core` CSS registration and file watching to reload
  changed styles.
- Generate dynamic CSS for volatile values, such as Pomodoro background color,
  into a small separate stylesheet that can be reloaded independently.
- Keep GNOME and niri side effects in `rsynapse-runtime`, not in framework
  crates.

The AGS `@define-color` to SCSS variable bridge is migration-specific. A Rust
port can either preserve that bridge through a small preprocessor or flatten the
theme contract into explicit CSS variables/design tokens used by the migrated
stylesheets.

Desktop theme side effects to preserve:

- Toggle `org.gnome.desktop.interface color-scheme`.
- Set `gtk-theme` to the dark or light variant.
- Update `~/.config/niri/theme.kdl` symlink from `theme_dark.kdl` or
  `theme_light.kdl`.
- Sync accent color into `~/.local/share/themes/accent-color.css`.
- Set GTK icon theme to `Material` for shell widgets.

Candidate missing shell feature:

- [dynamic-css-reload.md](../../missing-shell-features/dynamic-css-reload.md)
- [stylesheet-build-pipeline.md](../../missing-shell-features/stylesheet-build-pipeline.md)

## Pomodoro Provider And DND Side Effects

Move `org.gnome.Pomodoro` access into a typed provider or rsynapse-local
service:

```rust
pub struct PomodoroState {
    pub state: PomodoroPhase,
    pub elapsed: f64,
    pub duration: f64,
    pub is_paused: bool,
}
```

Provider behavior:

- Read initial `State`, `Elapsed`, `IsPaused`, and `StateDuration`.
- Watch D-Bus property changes.
- Normalize missing or `"null"` state to `None`.
- Expose control methods for start, stop, pause, skip, and toggle.

Runtime side effects:

- On transition into `Pomodoro`, enable swaync DND and send AutoRemote
  `dnd_on`.
- On transition into short break, long break, or none, disable swaync DND and
  send AutoRemote `dnd_off`.
- During a break, when elapsed/duration first reaches 0.5, send AutoRemote
  `break_ends`.
- Avoid repeated side effects by tracking previous phase and break notification
  threshold state.

Implementation can initially keep `scripts/dnd.sh` as an external command.
Longer term, call swaync and AutoRemote through explicit Rust helpers so errors
are observable and testable.

Candidate missing shell feature:

- [pomodoro-provider.md](../../missing-shell-features/pomodoro-provider.md)
- [side-effect-runner.md](../../missing-shell-features/side-effect-runner.md)

## Hints Mode And Triggerhappy

Keep the external triggerhappy approach unless a compositor-native key-state
source becomes available:

- Install a user service equivalent to
  `ags-super-hints-triggerhappy.user.service`.
- Replace `ags request hints active true|false` with `rsynapsectl hints
  active true|false`.
- Preserve the runtime-dir state file and lock behavior so left and right Meta
  keys combine into one active state.
- Expose hints mode as a typed provider or shared runtime state consumed by the
  widgets that render hints.

Candidate missing shell feature:

- [shared-runtime-state.md](../../missing-shell-features/shared-runtime-state.md)
- [keyboard-hints-bridge.md](../../missing-shell-features/keyboard-hints-bridge.md)

## Agent Approval Auto-Open

Keep the auto-open rule outside the approval overlay component:

- Watch selected workspace.
- Resolve windows associated with that workspace.
- Resolve `window-agent-session` targets for those windows.
- Compare linked session ids with agent sessions that require attention and
  have a pending prompt.
- Open approval UI for the first matching session, deduplicated by
  `session_id + pending_prompt`.

This belongs in `rsynapse-runtime` or the approval binary, not in `shell/core`.
If it remains in the approval binary, expose only a typed command or internal
message that asks the overlay to show a session.

Candidate missing shell feature:

- [derived-locus-collections.md](../../missing-shell-features/derived-locus-collections.md)
- [cross-widget-actions.md](../../missing-shell-features/cross-widget-actions.md)

## Shared Helper Migration

AGS helper behavior maps to ordinary Rust/provider concepts:

- `diffs()` becomes monitor-set diffing by stable monitor id.
- `withPrevious()` becomes local state in a stream consumer or a small stream
  helper.
- `binding()` and `bindAs()` become `Provider<T>` streams feeding Relm4 model
  messages and `#[watch]`/generated `#[bind]` setters.
- `disposeOnDestroy()` becomes component/window-owned subscriptions with
  cancellation tokens.
- `fromFile()` becomes a file-watch provider.
- `fromJsonProcess()` and `execPeriodically()` become process providers or
  native Rust providers.

Do not port RxJS as a runtime dependency. Use `providers::Provider<T>`, Tokio
streams, Relm4 messages, cancellation tokens, and normal Rust state.

Candidate missing shell feature:

- [file-watch-provider.md](../../missing-shell-features/file-watch-provider.md)
- [process-output-provider.md](../../missing-shell-features/process-output-provider.md)

## Script Migration Notes

Keep or replace scripts based on risk:

- Keep initially: `dnd.sh`, `super-hints-trigger`, systemd service,
  triggerhappy config.
- Replace with Rust providers/helpers: `cpu.sh`, `ram.sh`, `sysstats.sh`,
  `ble_battery`.
- Replace or make build-time: `findstyles.sh`.
- Keep as a command helper if still needed: `suspend.sh`.
- Replace with Rust or keep as a side-effect helper: `sync_accent.sh`.

The scripts are part of the consumer shell behavior. They should not become
framework APIs unless multiple consumers need a generic process provider or
side-effect runner.

## Initial Model Sketches

Runtime-level state can stay small and typed:

```rust
pub struct RuntimeState {
    pub active_monitor: Option<MonitorId>,
    pub monitors: Vec<MonitorInfo>,
    pub hints_active: bool,
    pub color_scheme: ColorScheme,
    pub pomodoro: PomodoroState,
}
```

Widget-facing models should remain in their widget crates or modules. For
example, OSD and agent approvals consume `active_monitor`; bar consumes one
`MonitorInfo`; hints-rendering widgets consume `hints_active`.

## Migration Order

1. Add `rsynapsectl` and a minimal request service for `scheme-toggle` and
   `hints`.
2. Port CSS registration with static compiled CSS, then add dynamic Pomodoro
   CSS and development reload.
3. Port monitor lifecycle for per-monitor bars.
4. Port active-monitor provider and singleton overlays.
5. Port Pomodoro provider and DND side effects.
6. Rewire triggerhappy to `rsynapsectl`.
7. Port agent approval auto-open once Locus collection helpers and agent
   session providers are available.

## Open Questions

- Should `rsynapse-runtime` be a long-lived coordinator process, or should each
  widget binary own the subset of runtime policy it needs?
- What service name should replace `ags request` for user commands?
- Should dynamic CSS be generated at runtime, or should Pomodoro state update
  GTK CSS classes/properties while static CSS handles colors?
- Should AutoRemote remain shell-script based, or should it become a typed Rust
  HTTP side-effect helper?
- Which monitor identity is stable enough for window lifecycle across hotplug,
  scale changes, and compositor restarts?
