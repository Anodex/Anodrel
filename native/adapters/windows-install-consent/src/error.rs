//! Closed safe failures while showing the native initial-install confirmation.

use std::fmt;

/// Windows could not display the owned initial-install confirmation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitialInstallConsentError {
    /// Windows did not return a valid result from the native confirmation.
    DisplayFailed,
}

impl fmt::Display for InitialInstallConsentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the installer confirmation could not be displayed")
    }
}

impl std::error::Error for InitialInstallConsentError {}

#[cfg(test)]
mod tests {
    use super::InitialInstallConsentError;

    #[test]
    fn consent_failure_does_not_expose_native_status() {
        assert_eq!(
            InitialInstallConsentError::DisplayFailed.to_string(),
            "the installer confirmation could not be displayed"
        );
    }
}
