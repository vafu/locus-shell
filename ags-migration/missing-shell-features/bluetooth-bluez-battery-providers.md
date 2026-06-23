# Bluetooth And BlueZ Battery Providers

## Needed By

- [Bar](../migration/widgets/bar.md)

## Status

The Rust bar now shows Bluetooth status and grouped keyboard/audio/pointer
device indicators through locusfs BlueZ/UPower data.

## Direction

Remaining provider work:

- Move AGS' dual-battery behavior into the locusfs BlueZ projection. AGS merged
  UPower HID batteries with GATT Battery Service data; the shell should consume
  a normalized none/single/dual battery model instead of matching that itself.
- Keep device-type classification in `rsynapse-shell` unless it becomes
  generally reusable.
