//! Closed safe failures from the direct elevated-update handoff.

use std::fmt;

/// A locked installer image could not complete the direct handoff lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateHandoffError {
    /// Windows reported that the operator declined the UAC consent request.
    UserDeclined,
    /// Windows could not begin the fixed elevated launch.
    LaunchFailed,
    /// Windows began the launch but did not return a process handle.
    ProcessUnavailable,
    /// Windows could not safely wait for or inspect the started process.
    ProcessWaitFailed,
}

impl fmt::Display for UpdateHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UserDeclined => "the update was not approved",
            Self::LaunchFailed => "the elevated update could not be started",
            Self::ProcessUnavailable => "the elevated update process could not be observed",
            Self::ProcessWaitFailed => "the elevated update process could not be observed",
        })
    }
}

impl std::error::Error for UpdateHandoffError {}

#[cfg(test)]
mod tests {
    use super::UpdateHandoffError;

    #[test]
    fn handoff_errors_are_safe_operator_messages() {
        assert_eq!(
            UpdateHandoffError::UserDeclined.to_string(),
            "the update was not approved"
        );
        assert_eq!(
            UpdateHandoffError::LaunchFailed.to_string(),
            "the elevated update could not be started"
        );
    }
}
