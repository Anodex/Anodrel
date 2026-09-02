#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! One opaque first-party Windows update flow for native host composition.
//!
//! The flow orders cache recovery, signed policy discovery, private download,
//! locked image acceptance, and fixed UAC handoff without exposing their raw
//! choices. It has no application protocol, automatic scheduling, consent UI,
//! progress reporting, restart, or installation-proof surface. See
//! `docs/UPDATE_FLOW.md` and Decision 0171.

mod error;
mod flow;

pub use error::{UpdateImagePreparationError, UpdateLaunchError, UpdateOfferError};
pub use flow::{AvailableUpdate, ReadyUpdate, discover_current_update};
