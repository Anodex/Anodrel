#![forbid(unsafe_code)]

//! Host-owned lifecycle for one Linux development child and private transport.
//!
//! This joins only existing Linux launch and transport mechanics. It is not a
//! Linux application host, product launcher, window API, or protocol surface.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{
    LinuxDevelopmentSessionError, RunningLinuxDevelopmentSession, start_development_session,
};
