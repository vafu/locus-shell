# AGS Inventory

## Entrypoints

- `/home/v47/.config/ags/app.ts`: starts AGS, prepares theme and commands,
  creates per-monitor bars, creates monitor-bound overlays, and wires app-level
  side effects.
- `/home/v47/.config/ags/style/style.ts`: compiles AGS SCSS and GTK color
  definitions into a runtime CSS file.

## Top-Level Widgets

- `bar`: bottom anchored per-monitor status bar.
- `osd`: monitor-bound on-screen volume and brightness progress overlay.
- `agent-approvals`: approval request overlay backed by AgentDBus status.
- `app-runtime`: non-visual runtime behavior shared across widgets.

## Bar Modules

- `widgets/bar/index.tsx`: bar window layout and left/center/right grouping.
- `widgets/bar/locus.tsx`: workspace buttons and workspace status display.
- `widgets/bar/window_indicators.tsx`: selected monitor workspace/window
  indicators.
- `widgets/bar/bzbus.tsx`: Bazel/build status summary.
- `widgets/bar/agent.tsx`: agent session status and attention state.
- `widgets/bar/audio_route.tsx`: audio route popover and volume indicator.
- `widgets/bar/bt_status.tsx`: Bluetooth device and battery indicator.
- `widgets/bar/indicators.tsx`: system stats, date/time, battery, power
  profile, Ethernet, and Wi-Fi indicators.
- `widgets/bar/mpris.tsx`: media playback status.
- `widgets/bar/tray.tsx`: system tray.
- `widgets/bar/panel-widgets.tsx`: reusable bar UI building blocks.

## Shared Widgets

- `widgets/materialicon.tsx`: icon rendering and theme/icon cache support.
- `widgets/circularstatus.ts`: level/progress indicator drawing.

## Services

- `services/locus.generated.ts`: generated Locus observable bindings.
- `services/locus.ts`: higher-level Locus workspace/window helpers.
- `services/workspace-status-provider.ts`: workspace aggregate models and
  window indicator summaries.
- `services/agent.ts`: AgentDBus session status service.
- `services/bzbus.ts`: build invocation status service.
- `services/bluetooth/*`: Bluetooth type and battery helpers.
- `services/brightness.tsx`: brightness service.
- `services/pomodoro.ts`: pomodoro state and durations.
- `services/requests.ts`: AGS request handler and internal request bus.
- `services/hints.ts`: super-hints mode state.

## Styles

- `style/base.scss`
- `style/common.scss`
- `style/bar.scss`
- `style/osd.scss`
- `style/agent.scss`
- `style/agent-approvals.scss`
- `style/gtk_colors.css`
- `style/dyn.css`

## Excluded Files

These files exist in the AGS tree but are not part of the Rust migration
scope:

- `widgets/rsynapse/*`, `services/rsynapse.ts`, `style/rsynapse.scss`: unused
  launcher/search surface.
- `services/remarked.ts`: no reachable import from `app.ts`.
- `services/agent-stats.ts`: no reachable import from `app.ts`.
- `services/agent-session-window.ts`: registers a request handler only if
  imported; no reachable module imports it.
- `commands.ts`: `bindCommands()` is called but currently empty.

## Scripts And System Integration

- `scripts/dnd.sh`: DND integration used by pomodoro side effects.
- `scripts/sysstats.sh`: system stats sampled by the bar.
- `scripts/cpu.sh`, `scripts/ram.sh`: lower-level stats helpers.
- `scripts/suspend.sh`: power command helper.
- `scripts/sync_accent.sh`: theme/accent sync.
- `scripts/super-hints-trigger`: super-hints trigger helper.
- `triggerhappy/super-hints.conf`: triggerhappy key config.
- `systemd/ags-super-hints-triggerhappy.user.service`: triggerhappy service.
