#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Direct Windows staging for one already verified update installer image.
//!
//! This internal adapter reloads installed identity facts, streams one signed
//! catalogue image into a fresh private file, and verifies its byte descriptor.
//! It has no application protocol, discovery, launch, elevation, or install
//! surface. See `docs/UPDATE_DELIVERY.md` and Decision 0167.

mod acceptance;
mod candidate;
mod discovery;
mod discovery_error;
mod download;
mod error;
mod file;

pub use acceptance::{
    UpdateImageAcceptanceError, VerifiedDownloadedInstaller, verify_downloaded_update_image,
};
pub use candidate::{PreparedUpdateDownload, prepare_current_update_download};
pub use discovery::retrieve_current_update_download;
pub use discovery_error::UpdateCatalogueDiscoveryError;
pub use download::download_prepared_update;
pub use error::UpdateDownloadError;
pub use file::DownloadedInstaller;
