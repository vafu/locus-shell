//! Typed definitions for common provider sources.
//!
//! This crate contains reusable service definitions only. Runtime watching,
//! subscriptions, and transport behavior live in provider implementation crates
//! such as `dbus-provider`.
//!
//! Example:
//!
//! ```ignore
//! use common_providers::upower::{DISPLAY_DEVICE, DisplayDevice};
//!
//! let source = DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE);
//! ```

#[cfg(feature = "upower")]
pub mod upower;

#[cfg(all(test, feature = "upower"))]
mod test;
