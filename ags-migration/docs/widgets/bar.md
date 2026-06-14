# Bar Widget

## Source Files Reviewed

- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/widgets/bar/index.tsx`
- `/home/v47/.config/ags/widgets/bar/locus.tsx`
- `/home/v47/.config/ags/widgets/bar/window_indicators.tsx`
- `/home/v47/.config/ags/widgets/bar/bzbus.tsx`
- `/home/v47/.config/ags/widgets/bar/agent.tsx`
- `/home/v47/.config/ags/widgets/bar/audio_route.tsx`
- `/home/v47/.config/ags/widgets/bar/bt_status.tsx`
- `/home/v47/.config/ags/widgets/bar/indicators.tsx`
- `/home/v47/.config/ags/widgets/bar/mpris.tsx`
- `/home/v47/.config/ags/widgets/bar/tray.tsx`
- `/home/v47/.config/ags/widgets/bar/panel-widgets.tsx`
- `/home/v47/.config/ags/services/locus.ts`
- `/home/v47/.config/ags/services/workspace-status-provider.ts`
- `/home/v47/.config/ags/services/agent.ts`
- `/home/v47/.config/ags/services/bzbus.ts`
- `/home/v47/.config/ags/services/bluetooth/*`
- `/home/v47/.config/ags/widgets/materialicon.tsx`
- `/home/v47/.config/ags/widgets/circularstatus.ts`
- `/home/v47/.config/ags/style/bar.scss`

## Responsibilities

The bar is a per-monitor, bottom-anchored layer surface with exclusive screen
space. It appears on every connected monitor and is destroyed when that monitor
is removed.

The left side shows workspaces assigned to the monitor. Each workspace has a
collapsed project/workspace icon, a revealable title area, a tooltip, current
workspace state, attention/working/complete state, and optional super-hints
number badge. Workspace labels prefer project display metadata when a workspace
has a project, otherwise they show the workspace name/index.

The center shows compact window tiles for the active workspace on that monitor.
Plain windows show an application icon. Agent windows show a project/agent icon,
context usage level, subagent badge, active state, urgent/attention state, and a
tooltip summarizing model, state, context, and subagents.

The right side shows:

- MPRIS media status with a temporarily revealed artist/title label and an audio
  route menu button.
- Bazel/build status from Locus build invocation nodes, including offline, idle,
  running, failed, and finished states.
- CPU and RAM levels sampled every three seconds.
- System tray menu buttons.
- Power profile indicator that cycles active profile when clicked.
- Bluetooth status, connected device count, and hover-revealed battery widgets
  for keyboard, audio, and pointing/tablet devices.
- Audio output volume icon and audio route popover.
- Wired network state and speed.
- Wi-Fi SSID and icon.
- Battery icon and percentage tooltip.
- Clock button with date tooltip that toggles the notification center.

Agent widgets used inside workspace expansion expose status, selected state,
attention reasons, context level, subagent count, and a popover with model,
branch, cwd, cost, context, and pending elicitation response buttons.

## Visual Behavior

The bar background is transparent with bottom spacing. Content is grouped into
compact rounded blocks with elevation, 28-32 px minimum height, and restrained
spacing. Blocks animate color and outline changes quickly.

Workspace and status modules use pill-like grouped controls. Secondary controls
slide open on hover or while their popover is visible. Current workspace and
active window states use outlines rather than large fills. Inactive non-urgent
window tiles are partially dimmed.

Attention and urgency use warm/danger colors and animation. Agent thinking,
tool-use, compacting, and attention states animate the agent icon. Running
bzbus state spins the build icon; failed and finished builds recolor the block.

Badges are small rounded overlays with strong contrast. Workspace hint badges
are muted; agent subagent and notification-like badges use stronger accent or
warning colors.

Material Symbols are used for semantic controls and project/agent status.
Desktop/status sources use themed symbolic icons where available. Custom level
indicators render thin line or arc progress with stage-based colors for normal,
warn, high, danger, and critical ranges.

