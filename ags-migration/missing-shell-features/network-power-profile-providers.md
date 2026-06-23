# Network And Power Profile Providers

## Needed By

- [Bar](../migration/widgets/bar.md)

## Status

Implemented through locusfs-backed sources:

- NetworkManager wired and Wi-Fi indicators.
- PowerProfiles active profile display and cycling through the locusfs D-Bus
  `powerprofiles` projection and writable `ActiveProfile` property.

## Direction

Keep display and cycling policy in `rsynapse-shell`. Promote only stable
service contracts if another consumer needs the same view model.
