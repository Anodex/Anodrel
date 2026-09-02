#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct Windows elevation for one native-approved first installation.
//!
//! This adapter consumes the opaque native approval and asks Windows to run
//! only the current installer’s fixed `install` command with UAC. It has no
//! downloader, application protocol, installer path, argument, policy, or
//! shell-selection surface. See `docs/INSTALL_HANDOFF.md` and Decision 0179.

mod error;
mod process;
mod raw;

pub use error::InitialInstallHandoffError;
pub use process::{
    CompletedElevatedInitialInstall, ElevatedInitialInstallExit, ElevatedInitialInstallProcess,
    begin_elevated_initial_install,
};
