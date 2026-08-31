//! Fixed-grant development native UI-session entry points.
//!
//! Configuration owns the closed template grant sets. Lifecycle owns the
//! private child, pipe-worker, and host-window sequence.

mod config;
mod lifecycle;

pub(crate) use config::DevelopmentUiSessionConfig;
pub(crate) use lifecycle::{run, run_with_window_observer};
