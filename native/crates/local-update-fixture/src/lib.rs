#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Fixed local artifacts for the Windows signed-update acceptance fixture.
//!
//! This crate knows no listener, certificate, machine policy, installer,
//! command line, environment setting, or application protocol. It names only
//! the immutable fixture identity, HTTPS request targets, and two checked
//! publication files required by Decision 0221.

mod publication;
mod routing;

pub use publication::{
    APPLICATION_ID, CATALOGUE_REQUEST_TARGET, FixturePublication, FixturePublicationError,
    INITIAL_VERSION, INSTALLER_REQUEST_TARGET, LOCALHOST, PORT, UPDATE_VERSION, publication_root,
};
pub use routing::{FixtureResource, resolve_request};
