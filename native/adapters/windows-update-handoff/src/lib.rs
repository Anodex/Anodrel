#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct Windows elevation for an already locked exact update installer.
//!
//! This adapter consumes the opaque output of the signed-image gate and asks
//! Windows to run only its fixed `update` command with UAC. It has no downloader,
//! application protocol, endpoint, cache, installer-path, argument, or shell
//! selection surface. See `docs/UPDATE_HANDOFF.md` and Decision 0169.

mod error;
mod process;
mod raw;

pub use error::UpdateHandoffError;
pub use process::{ElevatedUpdateExit, ElevatedUpdateProcess, begin_elevated_update};
