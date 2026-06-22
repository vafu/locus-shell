# System Indicators Migration Plan

## Source Files Reviewed

- `/home/v47/.config/ags/widgets/bar/index.tsx`
- `/home/v47/.config/ags/widgets/bar/indicators.tsx`
- `/home/v47/.config/ags/widgets/bar/audio_route.tsx`
- `/home/v47/.config/ags/widgets/bar/panel-widgets.tsx`
- `/home/v47/.config/ags/widgets/circularstatus.ts`
- `/home/v47/.config/ags/style/bar.scss`
- `/home/v47/.config/ags/scripts/sysstats.sh`

## Scope

Port the essential right-side panel cluster into `rsynapse-shell`.

Currently implemented:

- clock/date button
- battery indicator
- NetworkManager Wi-Fi indicator
- NetworkManager wired Ethernet indicator
- PipeWire default output indicator
- CPU/memory dual level bar
- Bluetooth status and grouped device indicators

Still pending:

- PowerProfiles indicator and profile cycling.
- StatusNotifier tray.
- MPRIS.
- full audio route popover/actions.
- exact AGS sizing, spacing, and hover behavior parity.

## AGS Shape To Match

Right side order in AGS:

```text
MPRIS, SysStats, [Tray, PowerProfiles, Bluetooth, Audio, Eth, Wifi, Battery], Clock
```

Current Rust bar order:

```text
SysStats, [Bluetooth, Audio, Eth, Wifi, Battery], Clock
```

Use the existing Rust bar's `system-cluster` region. Keep indicators compact:
plain GTK images for symbolic system icons, Material icons only where AGS used
`MaterialIcon`, and no visible explanatory text.

Shared visual classes to preserve:

- `barblock` for grouped right-side regions.
- `panel-widget` on standalone icon/level widgets.
- `panel-button flat circular` on clickable buttons.
- `battery` on the arc level indicators used by dual level meters.
- `audio-route` and `audio-route-title` when the audio popover is ported.

## Target DTOs

Keep widget models plain and source-driven:

```rust
pub struct ClockView {
    pub time: String, // "%H:%M"
    pub date: String, // "%a %b %d"
}

pub struct SysStatsView {
    pub cpu: u8,
    pub mem: u8,
}

pub struct BatteryView {
    pub present: bool,
    pub percent: u8,
    pub state: BatteryState,
    pub icon: String,
    pub tooltip: String,
}

pub struct WifiView {
    pub visible: bool,
    pub icon: String,
    pub tooltip: String,
}

pub struct AudioView {
    pub icon: String,
    pub tooltip: String,
}

pub struct PowerProfileView {
    pub active_profile: String,
    pub icon: String,
}
```

## Source Contracts

All sources should return `shell_core::source::Observable<T>` and live beside
the widget that consumes them, not in a top-level `rsynapse-shell/src/sources`
module or in `shell/core`.

### Clock

- Source: local timer observable.
- Tick interval: 1 second.
- Format: time `%H:%M`, date `%a %b %d`.
- Button action: `swaync-client -t` remains a command action. Keep the source
  itself pure; the click handler can spawn the command later.

### SysStats

- Source: local `/proc` reads or the existing `scripts/sysstats.sh` behavior.
- Emit every 3 seconds.
- Match AGS thresholds:

```text
0 normal, 35 warn, 50 high, 80 danger, 90 critical
```

- Render with the AGS `DualIndicator` shape: left CPU arc, center `memory`
  Material icon, right memory arc with `curveDirection: start`.
- Prefer direct `/proc/stat` and `/proc/meminfo` reads in Rust over shelling out
  once implementation starts. Use the script only as a compatibility reference.

### Battery

- Current Rust source already reads BAT1 through locusfs:
  `dbus-service/upower/object/battery_BAT1/`.
- Extend it to read `IconName` from the same object. This should be the primary
  icon source because AGS used Astal's `battery_icon_name`, which maps to the
  UPower symbolic battery icon behavior.
- Continue reading:
  - `Percentage`
  - `State`
  - `IsPresent`
  - `IconName`
- Watch the BAT1 object directory and refresh all fields on any object event.
- Fallback only if `IconName` is absent:
  - charging or pending charge: `battery-good-charging-symbolic`,
    `battery-low-charging-symbolic`, etc. by percentage bucket.
  - full: `battery-full-symbolic`.
  - discharging: `battery-full-symbolic`, `battery-good-symbolic`,
    `battery-medium-symbolic`, `battery-low-symbolic`, or
    `battery-caution-symbolic`.
  - missing/unknown: `battery-missing-symbolic`.
- Tooltip should match AGS's compact behavior initially: the percentage as text.

### Wi-Fi

- Source: locusfs D-Bus object projection for NetworkManager.
- Model fields:
  - SSID for tooltip.
  - icon name equivalent to Astal `wifi.iconName`.
  - visible/enabled state if there is no Wi-Fi device.
- Wired Ethernet is displayed separately from the same NetworkManager-backed
  source family.
- Do not add a separate NetworkManager D-Bus runtime to `rsynapse-shell`; D-Bus
  should continue to come through locusfs for this migration.

### Audio

- AGS used WirePlumber/AstalWp, not D-Bus. The shell consumes a locusfs
  PipeWire projection instead of linking to WirePlumber directly.
- Required v1 locusfs contract:

```text
/pipewire/default/sink -> ../sink/<sink-id>
/pipewire/sink/<sink-id>/description
/pipewire/sink/<sink-id>/muted
/pipewire/sink/<sink-id>/volume-percent
/pipewire/sink/<sink-id>/icon-name
```

- The immediate essential is the default output volume icon:
  - `icon-name` is preferred when exposed by the plugin.
  - fallback icon is calculated from `muted` and `volume-percent`.
  - tooltip is default output description plus volume/mute state, fallback
    `Audio Output`.
- The indicator stays hidden until `/pipewire/default/sink` can be resolved.
- Full route popover:
  - list current speakers
  - group by PipeWire device id once locusfs exposes device id and
    `priority.session`
  - default output first
  - selecting a row sets that sink default
- Implemented in `rsynapse-shell` against the current locusfs PipeWire sink
  nodes. It currently lists sinks directly because the projection does not yet
  expose AGS' `pw-dump` route grouping metadata.
- Known backend issue: the locusfs PipeWire plugin debounces `pactl subscribe`
  bursts and then publishes one full snapshot, so rapid volume changes collapse
  before the shell observes them.

### Power Profiles

- Preferred source: locusfs D-Bus projection of `power-profiles-daemon`.
- Read active profile and icon name equivalent to Astal
  `AstalPowerProfiles.iconName`.
- Button action cycles profiles in daemon order:

```text
profiles[(current_index + 1) % profiles.len()]
```

- Keep the source read-only first if locusfs does not expose method calls yet.
  Add the UI button and model, then wire the click action when a command/method
  path exists.

## Implementation Steps

Completed:

1. Added widget-local source modules for clock, sysstats, battery, network,
   audio, and Bluetooth.
2. Added compact right-side widgets for clock, sysstats, battery, wired,
   Wi-Fi, audio, and Bluetooth.
3. Updated `widgets/bar/mod.rs` right cluster order to:

```text
SysStats, [Bluetooth, Audio, Eth, Wifi, Battery], Clock
```

Remaining:

1. Add `power_profiles.rs` with `power_profile()` DTO and click/action wiring
   once method support exists.
2. Add StatusNotifier tray.
3. Add live locusfs MPRIS projection; the bar-side consumer is present and
   expects `/mpris/player/*` player nodes with metadata/playback properties.
4. Continue visual parity checks against AGS screenshots.
5. Verify code changes with `cargo fmt --check`, `cargo check -p
   rsynapse-shell`, and the existing shell test set.

## Dependencies To Confirm Before Coding

- locusfs paths for power profiles active profile and available profiles.
- locusfs method/action path for cycling PowerProfiles.
- locusfs or command action path for PipeWire default sink changes.
- StatusNotifier/AppIndicator and DBusMenu source shape.
- MPRIS source shape.
- Whether `swaync-client -t` is acceptable as a direct click command in
  `rsynapse-shell` or should go through a command action helper first.

## Deferred

- PowerProfiles.
- Tray, MPRIS, and audio route popover/actions.
- Notification center state beyond the clock button toggle.
