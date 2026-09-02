//! Closed safe failures while showing the native update consent prompt.

use std::fmt;

/// Windows could not display the owned update confirmation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateConsentError {
    /// Windows did not return a valid result from the native confirmation.
    DisplayFailed,
}

impl fmt::Display for UpdateConsentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the update confirmation could not be displayed")
    }
}

impl std::error::Error for UpdateConsentError {}

#[cfg(test)]
mod tests {
    use super::UpdateConsentError;

    #[test]
    fn consent_failure_does_not_expose_native_status() {
        assert_eq!(
            UpdateConsentError::DisplayFailed.to_string(),
            "the update confirmation could not be displayed"
        );
    }
}
