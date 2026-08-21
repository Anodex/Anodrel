//! Bounded text-only response values.

use std::fmt;

/// Maximum UTF-8 bytes in one text-fetch response body.
pub const MAX_NETWORK_TEXT_BYTES: usize = 32 * 1024;

/// One protocol-representable HTTP status and UTF-8 text response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkTextResponse {
    status_code: u16,
    text: String,
}

impl NetworkTextResponse {
    /// Validates a text response before it can cross the platform boundary.
    pub fn new(
        status_code: u16,
        text: impl Into<String>,
    ) -> Result<Self, NetworkTextResponseError> {
        if !(100..=599).contains(&status_code) {
            return Err(NetworkTextResponseError::InvalidStatusCode);
        }
        let text = text.into();
        if text.len() > MAX_NETWORK_TEXT_BYTES {
            return Err(NetworkTextResponseError::TooLarge);
        }
        Ok(Self { status_code, text })
    }

    /// Returns the HTTP response status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns the complete bounded UTF-8 text response body.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// A native response could not be represented by the public text contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkTextResponseError {
    /// The native HTTP status was outside the public 100 through 599 range.
    InvalidStatusCode,
    /// The complete UTF-8 body exceeded the fixed response limit.
    TooLarge,
}

impl fmt::Display for NetworkTextResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidStatusCode => "network response status is not representable",
            Self::TooLarge => "network response text exceeds the fixed size limit",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NetworkTextResponseError {}

#[cfg(test)]
mod tests {
    use super::{MAX_NETWORK_TEXT_BYTES, NetworkTextResponse, NetworkTextResponseError};

    #[test]
    fn response_requires_a_representable_status_and_bounded_text() {
        assert_eq!(
            NetworkTextResponse::new(99, "body"),
            Err(NetworkTextResponseError::InvalidStatusCode)
        );
        assert_eq!(
            NetworkTextResponse::new(600, "body"),
            Err(NetworkTextResponseError::InvalidStatusCode)
        );
        assert_eq!(
            NetworkTextResponse::new(200, "x".repeat(MAX_NETWORK_TEXT_BYTES + 1)),
            Err(NetworkTextResponseError::TooLarge)
        );
    }

    #[test]
    fn response_retains_only_status_and_text() {
        let response = NetworkTextResponse::new(404, "not found").expect("fixture is valid");
        assert_eq!(response.status_code(), 404);
        assert_eq!(response.text(), "not found");
    }
}
