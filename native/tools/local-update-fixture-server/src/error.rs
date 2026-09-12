//! Closed server failure categories.

use std::fmt;

/// One safe reason the local update fixture server could not continue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalUpdateFixtureServerError {
    /// The fixed local publication, signed candidate, or signed catalogue did not validate.
    Publication,
    /// Windows could not bind, receive, or reply through the fixed listener.
    Listener,
}

impl fmt::Display for LocalUpdateFixtureServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Publication => "the fixed local update publication is unavailable",
            Self::Listener => "the fixed local update listener is unavailable",
        })
    }
}

impl std::error::Error for LocalUpdateFixtureServerError {}
