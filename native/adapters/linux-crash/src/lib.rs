#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Linux store for bounded host crash records.
//!
//! It keeps the portable crash format host-only while using private
//! descriptor-anchored fixed files. See `docs/CRASH_REPORTS.md` and Decision
//! 0126.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{LinuxCrashInitializationError, LinuxCrashStore, RETAINED_RECORDS};
