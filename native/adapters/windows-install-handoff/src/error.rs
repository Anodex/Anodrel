//! Closed safe failures from the direct elevated initial-install handoff.

use std::fmt;

/// An approved first installation could not complete its UAC handoff lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitialInstallHandoffError {
    /// Windows could not provide a safe current installer image path.
    CurrentImageUnavailable,
    /// Windows reported that the person declined the UAC consent request.
    UserDeclined,
    /// Windows could not begin the fixed elevated launch.
    LaunchFailed,
    /// Windows began the launch but did not return a process handle.
    ProcessUnavailable,
    /// Windows could not safely wait for or inspect the started process.
    ProcessWaitFailed,
}

impl fmt::Display for InitialInstallHandoffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CurrentImageUnavailable => "the signed installer image is unavailable",
            Self::UserDeclined => "the installation was not approved",
            Self::LaunchFailed => "the elevated installation could not be started",
            Self::ProcessUnavailable | Self::ProcessWaitFailed => {
                "the elevated installation process could not be observed"
            }
        })
    }
}

impl std::error::Error for InitialInstallHandoffError {}

#[cfg(test)]
mod tests {
    use super::InitialInstallHandoffError;

    #[test]
    fn handoff_errors_are_safe_operator_messages() {
        assert_eq!(
            InitialInstallHandoffError::UserDeclined.to_string(),
            "the installation was not approved"
        );
        assert_eq!(
            InitialInstallHandoffError::ProcessWaitFailed.to_string(),
            "the elevated installation process could not be observed"
        );
    }
}
