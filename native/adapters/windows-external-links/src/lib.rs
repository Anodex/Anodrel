#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows opening of validated external HTTPS links.
//!
//! The adapter passes one validated URL to the system-associated handler. It
//! does not build a command line or expose native status. See
//! `docs/EXTERNAL_LINKS.md`.

mod raw;

use anodrel_external_links::{ExternalLink, ExternalLinkOpenError, ExternalLinkService};

/// Direct Windows external-link service.
#[derive(Debug, Default)]
pub struct WindowsExternalLinks;

impl ExternalLinkService for WindowsExternalLinks {
    fn open(&self, link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
        open(link)
    }
}

/// Opens one validated HTTPS link through the ordinary Windows association.
pub fn open(link: &ExternalLink) -> Result<(), ExternalLinkOpenError> {
    raw::open(link.as_str()).map_err(|_| ExternalLinkOpenError::Unavailable)
}
