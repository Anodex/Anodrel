#![forbid(unsafe_code)]

//! One host-owned Linux development child and fixed Wayland Lab view.
//!
//! This is a development diagnostic composition, not a Linux application host,
//! public window API, or product-launch surface. See docs/LINUX_WINDOW_SESSIONS.md
//! and Decision 0131.

#[cfg(all(target_os = "linux", target_endian = "little"))]
mod linux;

#[cfg(all(target_os = "linux", target_endian = "little"))]
pub use linux::{
    LinuxDevelopmentWindowError, LinuxDevelopmentWindowEvent, LinuxDevelopmentWindowSession,
};
