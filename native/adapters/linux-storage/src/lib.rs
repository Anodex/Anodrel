#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Linux adapter for one bounded application-state snapshot.
//!
//! The adapter derives an effective-account location, then keeps every
//! filesystem operation behind host-owned fixed names and descriptors. See
//! `docs/STORAGE.md` and Decision 0125.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{LinuxStorageInitializationError, LinuxStorageService};
