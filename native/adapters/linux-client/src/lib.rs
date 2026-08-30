#![forbid(unsafe_code)]

//! Strict Linux invitation handling and direct connection for one child.
//!
//! This crate accepts only a host-delivered ANLI invitation and opens only its
//! validated Linux abstract Unix socket. It cannot create a listener, choose an
//! endpoint, discover a host, or connect through TCP.

#[cfg(target_os = "linux")]
mod invitation;
#[cfg(target_os = "linux")]
mod stream;

#[cfg(target_os = "linux")]
pub use invitation::{LinuxBootstrapError, LinuxBootstrapInvitation};
#[cfg(target_os = "linux")]
pub use stream::LinuxClientStream;
