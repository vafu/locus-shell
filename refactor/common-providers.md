# common-providers Refactor Notes

## Current Role

`common-providers` contains feature-gated typed definitions for common D-Bus services. It should not own runtime watching, subscription behavior, transport policy, GTK/Relm4 integration, or shell widget policy.

## Public Surface

- `upower` feature enables UPower definitions.
- `upower::DisplayDevice` is the marker type for UPower's aggregate display battery device.
- `upower::DISPLAY_DEVICE` is a typed `dbus_provider::Object<DisplayDevice>`.
- `DisplayDevice` exposes typed D-Bus properties such as `PERCENTAGE`, `STATE`, `TIME_TO_EMPTY`, `TIME_TO_FULL`, and `IS_PRESENT`.

## Step-By-Step File Walkthrough

1. `provider/common/src/lib.rs` - crate-level responsibility statement, feature gate, and example. Read first to confirm this crate remains definitions-only.
2. `provider/common/src/upower.rs` - UPower object and property constants. Read next because it is the only current service module and defines the pattern for future services.
3. `provider/common/src/test.rs` - feature-gated binding test. Read last to confirm the typed definitions produce the expected `dbus-provider` binding metadata.

## Internal Structure

- Service modules are enabled through Cargo features.
- Definitions are marker types plus `dbus-provider` object/property descriptors.
- No runtime code is present in this crate.

## Behavior Summary

Consumers import definitions, bind a typed property to its typed object, and pass the resulting `PropertyBinding<T>` to `dbus-provider` or the neutral `providers` flow. With no features enabled, the crate exports no service modules.

## User Notes

None yet.

## Findings

- Good boundary: the crate contains definitions only and does not duplicate D-Bus watching logic.
- Risk: UPower `STATE` is exposed as raw `u32`. That is accurate at the D-Bus level but leaves every consumer to interpret state codes. A typed enum may be worth adding later if common service definitions are expected to be ergonomic rather than only low-level descriptors.
- Gap: tests only verify one UPower property binding. The remaining constants are simple, but future services should add one test per object or property group to catch copy/paste service/path/interface mistakes.

## Refactor Plan

1. Keep runtime watching in `dbus-provider`; do not add provider implementations here.
2. Add typed enums only when they reduce repeated consumer logic and can be mapped without hiding raw D-Bus behavior.
3. When new service modules are added, follow the same feature-gated marker/object/property pattern and add descriptor tests for each service object.

## Tests And Verification

- `cargo test -p common-providers --features upower` passes: 1 unit test, 1 ignored doctest.
- `cargo test -p common-providers` passes with no feature modules enabled: 0 unit tests, 1 ignored doctest.

## Open Questions

- Should common service definitions stay as raw D-Bus property descriptors only, or should they also provide lightweight typed enums for common integer-coded properties such as UPower state?
