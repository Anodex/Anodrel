#![deny(unsafe_op_in_unsafe_fn)]

//! Direct Linux development child launch with private standard-input bootstrap.
//!
//! This owns process mechanics only. Executable identity, policy, and lifecycle
//! belong to a future Linux host. See `docs/LINUX_LAUNCH.md` and Decision 0127.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{
    LaunchedProcess, LinuxBootstrapLaunchError, LinuxBootstrapProgram, LinuxProcessError,
    LinuxWaitError, launch,
};
