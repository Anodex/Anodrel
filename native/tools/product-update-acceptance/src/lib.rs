#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! One fixed-identity operator diagnostic for native Windows update acceptance.
//!
//! The diagnostic accepts no input and selects only the development product
//! fixture identity. It is not a product update API or a general launcher. See
//! `docs/PRODUCT_UPDATE_FIXTURE.md` and Decision 0174.

mod error;

use anodrel_windows_update_consent::{UpdateConsent, request_update_consent};
use anodrel_windows_update_handoff::UpdateHandoffError;
use anodrel_windows_updater::{UpdateLaunchError, discover_current_update};

pub use error::ProductUpdateAcceptanceError;

/// The one development fixture identity this diagnostic may update.
pub const FIXTURE_APPLICATION_ID: &str = "org.anodrel.product-fixture";

/// One closed result from an explicit manual update acceptance attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductUpdateAcceptanceOutcome {
    /// The person declined the fixed Anodrel confirmation before a download.
    ConsentDeclined,
    /// The person declined the separate Windows UAC confirmation.
    ElevationDeclined,
    /// The installed policy independently proved the selected signed update.
    Verified,
}

impl ProductUpdateAcceptanceOutcome {
    /// Returns the safe operator-facing completion message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::ConsentDeclined => "Update acceptance stopped before download.",
            Self::ElevationDeclined => "Update acceptance stopped before elevation.",
            Self::Verified => "Update acceptance verified.",
        }
    }

    /// Returns the process code for this closed outcome.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Verified => 0,
            Self::ConsentDeclined => 20,
            Self::ElevationDeclined => 21,
        }
    }
}

/// Runs the only supported fixed development update-acceptance sequence.
///
/// Invoke this only from the initial interactive thread of the operator-only
/// command runner. It reads only the fixture's installed policy, prompts for
/// the fixed native consent, then conducts the owned transfer, UAC handoff,
/// process observation, and policy proof in order. It does not restart an
/// application or turn a successful installer exit into verification.
pub fn run() -> Result<ProductUpdateAcceptanceOutcome, ProductUpdateAcceptanceError> {
    let offer = discover_current_update(FIXTURE_APPLICATION_ID)
        .map_err(ProductUpdateAcceptanceError::Offer)?;
    let offer =
        match request_update_consent(offer).map_err(ProductUpdateAcceptanceError::Consent)? {
            UpdateConsent::Approved(offer) => offer,
            UpdateConsent::Declined => return Ok(ProductUpdateAcceptanceOutcome::ConsentDeclined),
        };
    let ready = offer
        .download()
        .map_err(ProductUpdateAcceptanceError::Preparation)?;
    let process = match ready.begin_elevation() {
        Ok(process) => process,
        Err(UpdateLaunchError::HandoffInvalid(UpdateHandoffError::UserDeclined)) => {
            return Ok(ProductUpdateAcceptanceOutcome::ElevationDeclined);
        }
        Err(error) => return Err(ProductUpdateAcceptanceError::Launch(error)),
    };
    let completed = process
        .wait()
        .map_err(ProductUpdateAcceptanceError::Observation)?;
    completed
        .verify_selection()
        .map_err(ProductUpdateAcceptanceError::Postcondition)?;
    Ok(ProductUpdateAcceptanceOutcome::Verified)
}

#[cfg(test)]
mod tests {
    use super::{FIXTURE_APPLICATION_ID, ProductUpdateAcceptanceOutcome};

    #[test]
    fn the_runner_can_select_only_the_declared_fixture_identity() {
        assert_eq!(FIXTURE_APPLICATION_ID, "org.anodrel.product-fixture");
        assert!(anodrel_application::is_valid_application_id(
            FIXTURE_APPLICATION_ID
        ));
    }

    #[test]
    fn only_policy_proof_reports_success() {
        assert_eq!(ProductUpdateAcceptanceOutcome::Verified.exit_code(), 0);
        for outcome in [
            ProductUpdateAcceptanceOutcome::ConsentDeclined,
            ProductUpdateAcceptanceOutcome::ElevationDeclined,
        ] {
            assert_ne!(outcome.exit_code(), 0);
            assert!(outcome.message().contains("stopped"));
        }
    }
}
