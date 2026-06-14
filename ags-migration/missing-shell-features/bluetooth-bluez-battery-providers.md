# Bluetooth And BlueZ Battery Providers

## Needed By

- [Bar](../migration/widgets/bar.md)

## Gap

Bluetooth device status and battery data currently come from Astal plus custom
UPower/BlueZ probing.

## Direction

Create typed providers for BlueZ devices, UPower HID battery paths, and GATT
battery reads. Keep device-type classification in `rsynapse-shell` unless it
becomes generally reusable.

