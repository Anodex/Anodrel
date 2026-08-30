#![deny(unsafe_op_in_unsafe_fn)]

//! One-client, authenticated Linux abstract Unix-socket adapter for Anodrel.
//!
//! This crate is intentionally Linux-specific. `serve_one` performs stream
//! work on a host worker thread; it must not run on a future UI thread.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{
    InvitationError, LinuxPipeServer, LinuxPipeStopSignal, SessionInvitation, run_health_self_test,
};
