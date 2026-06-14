# Brightness Provider

## Needed By

- [OSD](../migration/widgets/osd.md)

## Gap

The OSD needs normalized brightness updates from the active backlight device.

## Direction

Start with an `rsynapse-shell` provider backed by `/sys/class/backlight` and
`brightnessctl`-compatible semantics. Consider a generic provider only after
write/control requirements are clear.

