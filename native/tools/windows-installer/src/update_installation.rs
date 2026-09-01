//! One no-argument composition of the owned signed machine-update gates.

use std::fmt;

use crate::machine_root::current_machine_application_root;
use crate::prepared::prepare_verified_signed_release;
use crate::{
    MachineRootError, PreparedReleaseError, PromotionError, PublicationError, SignedReleaseError,
    UpdatePreflightError, verify_current_signed_release, verify_current_update_candidate,
};

/// A new release selected by the fixed machine policy through an update transaction.
pub struct UpdatedRelease {
    published: crate::PublishedRelease,
}

impl fmt::Debug for UpdatedRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("UpdatedRelease")
            .field(&self.published)
            .finish()
    }
}

/// The current signed installer release could not complete a safe machine update.
#[derive(Debug)]
pub enum UpdateCurrentError {
    /// The current installer did not establish a publisher-continuous newer candidate.
    CandidateInvalid(UpdatePreflightError),
    /// The current installer could not be activated again before staging.
    CandidateRefreshInvalid(SignedReleaseError),
    /// The refreshed release differed from the verified candidate decision.
    CandidateChanged,
    /// The fixed machine root could not be established from the signed identity.
    MachineRootInvalid(MachineRootError),
    /// The refreshed release could not become a prepared private stage.
    PreparationFailed(PreparedReleaseError),
    /// The checked stage could not become a new version directory.
    PromotionFailed(PromotionError),
    /// The promoted record could not become the fixed selected machine policy.
    PublicationFailed(PublicationError),
}

impl fmt::Display for UpdateCurrentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CandidateInvalid(_) => "the signed update candidate is invalid",
            Self::CandidateRefreshInvalid(_) => {
                "the signed update candidate could not be refreshed"
            }
            Self::CandidateChanged => "the signed update candidate changed before staging",
            Self::MachineRootInvalid(_) => "the fixed machine installation root is invalid",
            Self::PreparationFailed(_) => "the signed update could not be prepared",
            Self::PromotionFailed(_) => "the signed update could not be promoted",
            Self::PublicationFailed(_) => "the signed update could not be selected",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UpdateCurrentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CandidateInvalid(error) => Some(error),
            Self::CandidateRefreshInvalid(error) => Some(error),
            Self::MachineRootInvalid(error) => Some(error),
            Self::PreparationFailed(error) => Some(error),
            Self::PromotionFailed(error) => Some(error),
            Self::PublicationFailed(error) => Some(error),
            Self::CandidateChanged => None,
        }
    }
}

/// Updates only from the current signed embedded release after a refreshed trust decision.
///
/// This accepts no path, application identity, version, publisher, registry
/// data, executable, certificate, network input, or policy. It refreshes the
/// current signed release after preflight and requires the same identity,
/// version, and publisher before private staging begins. It does not download,
/// create trust, launch a process, create application data, remove an existing
/// version, or establish a background update route.
pub fn update_current_signed_release() -> Result<UpdatedRelease, UpdateCurrentError> {
    let candidate =
        verify_current_update_candidate().map_err(UpdateCurrentError::CandidateInvalid)?;
    let release =
        verify_current_signed_release().map_err(UpdateCurrentError::CandidateRefreshInvalid)?;
    if !candidate.matches_manifest(release.release().manifest()) {
        return Err(UpdateCurrentError::CandidateChanged);
    }
    let root = current_machine_application_root(candidate.application_id())
        .map_err(UpdateCurrentError::MachineRootInvalid)?;
    let prepared = prepare_verified_signed_release(root.path(), release)
        .map_err(UpdateCurrentError::PreparationFailed)?;
    let promoted =
        crate::promote_prepared_release(prepared).map_err(UpdateCurrentError::PromotionFailed)?;
    let published =
        crate::publish_promoted_release(promoted).map_err(UpdateCurrentError::PublicationFailed)?;
    Ok(UpdatedRelease { published })
}

#[cfg(test)]
mod tests {
    use anodrel_windows_signature::SignatureError;

    use super::{UpdateCurrentError, update_current_signed_release};
    use crate::{SignedReleaseError, UpdatePreflightError};

    #[test]
    fn an_unsigned_current_image_stops_before_update_root_selection() {
        assert!(matches!(
            update_current_signed_release(),
            Err(UpdateCurrentError::CandidateInvalid(
                UpdatePreflightError::CandidateInvalid(SignedReleaseError::SignatureInvalid(
                    SignatureError::TrustRejected
                ))
            ))
        ));
    }
}
