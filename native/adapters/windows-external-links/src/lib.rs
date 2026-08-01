#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows opening of validated external HTTPS links.
//!
//! The adapter passes one validated URL to the system-associated handler. It
//! does not build a command line or expose native status. See
//! `docs/EXTERNAL_LINKS.md`.

mod raw;

use std::fmt;

use anodrel_external_links::ExternalLink;

/// Opens one validated HTTPS link through the ordinary Windows association.
pub fn open(link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
    raw::open(link.as_str()).map_err(|_| ExternalLinkOpenError::Unavailable)
}

/// A safe category for a Windows external-link opening failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalLinkOpenError {
    /// Windows could not hand the link to an associated handler.
    Unavailable,
}

impl fmt::Display for ExternalLinkOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("external link handler is unavailable")
    }
}

impl std::error::Error for ExternalLinkOpenError {}
