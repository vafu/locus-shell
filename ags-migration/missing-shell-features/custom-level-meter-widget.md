# Custom Level Meter Widget

## Needed By

- [Bar](../migration/widgets/bar.md)
- [OSD](../migration/widgets/osd.md)

## Gap

AGS uses a custom circular/arc level indicator for compact status display.

## Direction

Implement a GTK/Relm4 custom widget or drawing area in `rsynapse-shell`. Promote
to a reusable crate only after API and styling needs settle.

