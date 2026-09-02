#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! One opaque first-party Windows update flow for native host composition.
//!
//! The flow orders cache recovery, signed policy discovery, private download,
//! locked image acceptance, and fixed UAC handoff without exposing their raw
//! choices. It has no application protocol, automatic scheduling, consent UI,
//! progress reporting, or restart surface; its completed handoff can prove only
//! the fixed machine-policy postcondition. See `docs/UPDATE_FLOW.md` and
//! Decisions 0171 and 0172.

mod error;
mod flow;

pub use anodrel_windows_update_download::{UpdateCompletionError, VerifiedUpdateInstallation};
pub use anodrel_windows_update_handoff::{
    CompletedElevatedUpdate, ElevatedUpdateExit, ElevatedUpdateProcess,
};
pub use error::{UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError};
pub use flow::{AvailableUpdate, ReadyUpdate, discover_current_update};
