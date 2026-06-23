# StatusNotifier Tray Provider

## Needed By

- [Bar](../migration/widgets/bar.md)

## Gap

Tray item discovery, icons, menus, and activation behavior are now available in
the Rust bar through locusfs plugins.

## Direction

Implemented pieces:

- `../locusfs/plugins/statusnotifier` projects StatusNotifier items into
  locusfs.
- `../locusfs/plugins/dbusmenu` projects DBusMenu layout and writable
  activation nodes.
- `rsynapse-shell` consumes tray items as locusfs relations and renders DBusMenu
  menus as GTK popovers.

Remaining follow-up:

- verify dynamic DBusMenu layout updates and submenu edge cases.
- keep stale tray-item cleanup owned by the locusfs StatusNotifier projection,
  not by shell-side filtering.
