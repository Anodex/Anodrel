#![forbid(unsafe_code)]

//! Windows registered-application session setup.
//!
//! This adapter joins a validated, machine-selected application policy to one
//! owner-restricted Windows named-pipe session. Its optional interactive path
//! also creates the one grouped set of host-owned resources consumed by an
//! authenticated native window. It does not launch a process, deliver a
//! bootstrap invitation, select an application ID, or serve pipe I/O; callers
//! retain those lifecycle responsibilities.

mod error;
mod services;
mod session;
mod ui;

pub use error::RegisteredSessionError;
pub use session::{RegisteredUiSession, create_registered_session, create_registered_ui_session};
pub use ui::RegisteredSessionUi;

#[cfg(test)]
pub(crate) use session::create_session;
#[cfg(test)]
mod tests;
