#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! One host-owned direct Windows consent decision for a signed first install.
//!
//! This adapter displays only a fixed native confirmation for an opaque
//! `PreparedInitialInstall`. It has no application protocol, preference, path,
//! UAC, process, policy, or installation authority. See
//! `docs/INSTALL_CONSENT.md` and Decision 0178.

mod error;
mod raw;

use anodrel_windows_installer::PreparedInitialInstall;

pub use error::InitialInstallConsentError;

/// One approval produced only by the native initial-install confirmation.
pub struct ApprovedInitialInstall {
    prepared: PreparedInitialInstall,
}

impl ApprovedInitialInstall {
    /// Consumes the native approval for the fixed later UAC-handoff boundary.
    #[must_use]
    pub fn into_prepared(self) -> PreparedInitialInstall {
        self.prepared
    }
}

/// One explicit decision over the original opaque signed first installation.
pub enum InitialInstallConsent {
    /// The person approved the later fixed elevation and installation flow.
    Approved(Box<ApprovedInitialInstall>),
    /// The person declined; no elevation or installation stage has started.
    Declined,
}

/// Requests the fixed direct Windows confirmation for one prepared first install.
///
/// Call this on the installer UI thread. Approval creates the only opaque value
/// accepted by the later UAC handoff. This operation itself does not elevate,
/// launch, install, mutate policy, or retain a preference.
pub fn request_initial_install_consent(
    prepared: PreparedInitialInstall,
) -> Result<InitialInstallConsent, InitialInstallConsentError> {
    match raw::request(prepared.candidate_version())
        .map_err(|_| InitialInstallConsentError::DisplayFailed)?
    {
        raw::NativeConsent::Approved => Ok(InitialInstallConsent::Approved(Box::new(
            ApprovedInitialInstall { prepared },
        ))),
        raw::NativeConsent::Declined => Ok(InitialInstallConsent::Declined),
    }
}

#[cfg(test)]
mod tests {
    use super::InitialInstallConsent;

    #[test]
    fn decline_has_no_prepared_installation_or_follow_on_authority() {
        assert!(matches!(
            InitialInstallConsent::Declined,
            InitialInstallConsent::Declined
        ));
    }
}
