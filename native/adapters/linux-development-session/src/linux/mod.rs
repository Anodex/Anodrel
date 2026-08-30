//! Linux-only implementation of the host-owned development-session lifetime.

mod lifecycle;

pub use lifecycle::{
    LinuxDevelopmentSessionError, RunningLinuxDevelopmentSession, start_development_session,
};
