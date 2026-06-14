# OSD Widget

## AGS Sources Reviewed

- `/home/v47/.config/ags/widgets/osd/index.tsx`
- `/home/v47/.config/ags/widgets/osd/OSD.tsx`
- `/home/v47/.config/ags/services/brightness.tsx`
- `/home/v47/.config/ags/style/osd.scss`
- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/services/locus.ts`

## Responsibilities

The OSD is a transient overlay for system level changes. It currently reports:

- Screen brightness changes.
- Default speaker volume changes.

Each visible event contains:

- A normalized level value from `0.0` to `1.0`.
- A symbolic icon name that describes the current source or level.

Brightness uses a fixed `display-brightness-symbolic` icon. Audio uses the default speaker endpoint's current volume icon, such as the symbolic low, medium, high, or muted audio icon provided by the audio service.

The OSD follows the active monitor, not every monitor. The active monitor is resolved from the selected Locus output connector and mapped to the corresponding `Gdk.Monitor`. In the AGS startup path, OSD is created once after the first active monitor is available, and its monitor binding updates as the active monitor changes.

## Display Timing

Any brightness or audio event shows the OSD and starts a one-second hide timer. A newer event replaces the current content and restarts the hide timer.

The content has its own reveal state. The window remains visible briefly after the content is hidden so the crossfade-out can complete.

## Layout And Visual Behavior

The OSD is a bottom-anchored overlay window. It does not reserve screen space, does not take focus, and is styled through the `OSD` window class.

The visible content is a compact vertical stack:

- A large symbolic icon.
- A horizontal level bar below the icon.

The content shell has:

- Rounded corners with a `24px` radius.
- OSD background color from the GTK theme color mapping, currently `osd_bg` mapped to `@sidebar_bg_color`.
- Elevation shadow equivalent to `0px 4px 16px 4px` using the configured `shadow` color.
- Padding of `13px 16px`.
- Outer margin of `50px` from the bottom anchor area.

The icon has `12px` padding. The level bar has a fixed requested width of `100px`; its trough has `0.6rem` margin and its blocks have a minimum height of `2rem`.

The content reveal transition is a crossfade.
