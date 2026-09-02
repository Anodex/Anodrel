#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! One host-owned direct Windows consent decision for a signed update offer.
//!
//! This adapter displays only a fixed native confirmation for an opaque
//! `AvailableUpdate`. It has no application protocol, preference, endpoint,
//! path, UAC, transfer, process, or installation authority. See
//! `docs/UPDATE_CONSENT.md` and Decision 0173.

mod error;
mod raw;

use anodrel_windows_updater::AvailableUpdate;

pub use error::UpdateConsentError;

/// One explicit decision over the original opaque signed update offer.
pub enum UpdateConsent {
    /// The person approved the later private download and installation flow.
    Approved(AvailableUpdate),
    /// The person declined; no update stage has started.
    Declined,
}

/// Requests the fixed direct Windows confirmation for one signed update offer.
///
/// Call this on the native host's UI thread and only after an explicit user
/// update action. Approval returns the same opaque offer for its later download
/// stage. This operation itself does not download, elevate, launch, install,
/// mutate policy, or retain a preference.
pub fn request_update_consent(offer: AvailableUpdate) -> Result<UpdateConsent, UpdateConsentError> {
    match raw::request(offer.candidate_version()).map_err(|_| UpdateConsentError::DisplayFailed)? {
        raw::NativeConsent::Approved => Ok(UpdateConsent::Approved(offer)),
        raw::NativeConsent::Declined => Ok(UpdateConsent::Declined),
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateConsent;

    #[test]
    fn decline_has_no_embedded_offer_or_follow_on_authority() {
        assert!(matches!(UpdateConsent::Declined, UpdateConsent::Declined));
    }
}
