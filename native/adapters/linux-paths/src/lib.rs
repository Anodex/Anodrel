#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Linux lookup for host-owned application directories.
//!
//! The adapter derives a fixed current-user local-data root from the effective
//! account record. It does not read environment variables or touch a directory.
//! See docs/PATHS.md.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{LinuxPathsError, application_directories, host_directories, local_data_root};
