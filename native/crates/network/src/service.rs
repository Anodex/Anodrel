//! The portable host-owned network text service seam.

use std::fmt;

use crate::{NetworkTextResponse, NetworkUrl};

/// A host-owned service for one already validated HTTPS text fetch.
///
/// Implementations own all native network work and must enforce their
/// host-created origin policy before attempting a connection. They return no
/// native diagnostic detail through this seam.
pub trait NetworkTextService: fmt::Debug + Send {
    /// Fetches one bounded text response for a validated HTTPS URL.
    fn fetch_text(&self, url: &NetworkUrl) -> Result<NetworkTextResponse, NetworkTextServiceError>;
}

/// A safe failure category returned by a text-fetch service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkTextServiceError {
    /// The service, policy, network, or native request was unavailable.
    Unavailable,
    /// The native response could not satisfy the public status or text bounds.
    ResponseInvalid,
}

impl fmt::Display for NetworkTextServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => "network text service is unavailable",
            Self::ResponseInvalid => "network response is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NetworkTextServiceError {}
