#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, fixed-scope Wayland presentation for the Anodrel Linux Lab.
//!
//! This is a development diagnostic, not an application-window API. It owns
//! one compositor connection and two bounded shared-memory buffers. See
//! docs/LINUX_WINDOWING.md and Decision 0128.

#[cfg(all(target_os = "linux", target_endian = "little"))]
mod linux;

#[cfg(all(target_os = "linux", target_endian = "little"))]
pub use linux::{LAB_HEIGHT, LAB_WIDTH, LinuxWaylandError, LinuxWaylandLab};
