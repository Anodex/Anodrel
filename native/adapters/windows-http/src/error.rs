//! Closed failure categories for direct HTTPS transfer.

use std::fmt;

/// One direct Windows HTTPS transfer could not meet its bounded contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsHttpsError {
    /// The caller did not supply one valid bounded request contract.
    RequestInvalid,
    /// Windows could not create, configure, send, receive, or read the request.
    Unavailable,
    /// Windows returned a status that was not a representable HTTP status.
    ResponseInvalid,
    /// The response did not use the caller-required exact HTTP status.
    UnexpectedStatus,
    /// The response body exceeded the caller-selected bound.
    BodyTooLarge,
    /// The caller's bounded body consumer rejected one received chunk.
    ConsumerRejected,
}

impl fmt::Display for WindowsHttpsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RequestInvalid => "the HTTPS request is invalid",
            Self::Unavailable => "the HTTPS request is unavailable",
            Self::ResponseInvalid => "the HTTPS response is invalid",
            Self::UnexpectedStatus => "the HTTPS response status is unexpected",
            Self::BodyTooLarge => "the HTTPS response exceeds its fixed limit",
            Self::ConsumerRejected => "the HTTPS response could not be consumed",
        })
    }
}

impl std::error::Error for WindowsHttpsError {}
