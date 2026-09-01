//! Fixed machine-policy publication for one promoted Anodrel release.

use std::fmt;

use crate::PromotedRelease;

mod raw;

/// A promoted release whose record was written to the fixed machine policy key.
pub struct PublishedRelease {
    release: PromotedRelease,
}

/// A promoted update whose prior record was retained before fixed policy publication.
pub struct PublishedUpdate {
    release: PromotedRelease,
}

impl fmt::Debug for PublishedRelease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PublishedRelease")
            .field(&self.release)
            .finish()
    }
}

impl fmt::Debug for PublishedUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PublishedUpdate")
            .field(&self.release)
            .finish()
    }
}

/// A fixed machine-policy publication could not complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationError {
    /// The already validated record could not become one valid `REG_SZ` value.
    RecordEncodingInvalid,
    /// Windows denied the elevated policy write.
    AccessDenied,
    /// Windows machine policy was unavailable for the fixed write.
    RegistryUnavailable,
}

impl fmt::Display for PublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RecordEncodingInvalid => "the promoted release record cannot be published",
            Self::AccessDenied => "machine policy cannot be changed; run from an elevated shell",
            Self::RegistryUnavailable => "Windows machine policy is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PublicationError {}

/// An update could not retain the prior policy record before selecting its release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdatePublicationError {
    /// The existing selected record was unavailable for update retention.
    ExistingRecordUnavailable,
    /// The existing selected record was not one bounded valid `REG_SZ` value.
    ExistingRecordMalformed,
    /// The existing selected record changed while Windows was reading it.
    ExistingRecordChanged,
    /// The new promoted record could not become one valid `REG_SZ` value.
    NewRecordEncodingInvalid,
    /// Windows denied the elevated policy write or read.
    AccessDenied,
    /// Windows machine policy was unavailable for the fixed operation.
    RegistryUnavailable,
}

impl fmt::Display for UpdatePublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ExistingRecordUnavailable => {
                "the existing application policy record is unavailable"
            }
            Self::ExistingRecordMalformed => "the existing application policy record is malformed",
            Self::ExistingRecordChanged => {
                "the existing application policy record changed during read"
            }
            Self::NewRecordEncodingInvalid => "the promoted update record cannot be published",
            Self::AccessDenied => "machine policy cannot be changed; run from an elevated shell",
            Self::RegistryUnavailable => "Windows machine policy is unavailable",
        })
    }
}

impl std::error::Error for UpdatePublicationError {}

/// Publishes the one validated record selected by a promoted release.
///
/// This accepts neither a registry path nor record text. It writes only the
/// existing 64-bit machine policy value the Windows host reads, and does not
/// alter the release directory, remove a prior version, launch a process, or
/// create trust.
pub fn publish_promoted_release(
    release: PromotedRelease,
) -> Result<PublishedRelease, PublicationError> {
    raw::write_record(release.application_id(), release.install_record())?;
    Ok(PublishedRelease { release })
}

/// Retains the selected record, then publishes one promoted update record.
///
/// This accepts neither record text nor a value name. It copies only the fixed
/// current `record` to the fixed private `previous` value, then selects the
/// already validated promoted record. It does not create an initial policy,
/// alter package directories, launch a process, or remove older content.
pub fn publish_promoted_update(
    release: PromotedRelease,
) -> Result<PublishedUpdate, UpdatePublicationError> {
    raw::retain_current_then_write_update(release.application_id(), release.install_record())?;
    Ok(PublishedUpdate { release })
}

#[cfg(test)]
mod tests {
    use super::{PublicationError, UpdatePublicationError, raw};

    #[test]
    fn publication_uses_the_exact_fixed_machine_policy_location() {
        assert_eq!(
            raw::policy_path("org.anodrel.release-test"),
            "Software\\Anodrel\\Applications\\org.anodrel.release-test"
        );
        assert_eq!(raw::record_value_name(), "record");
        assert_eq!(raw::previous_value_name(), "previous");
    }

    #[test]
    fn records_require_one_terminal_utf16_nul_without_embedded_nuls() {
        assert_eq!(
            raw::encode_record("{}").expect("record encodes"),
            vec![123, 125, 0]
        );
        assert_eq!(
            raw::encode_record("{\0}"),
            Err(PublicationError::RecordEncodingInvalid)
        );
    }

    #[test]
    fn update_publication_keeps_distinct_safe_failure_categories() {
        assert_eq!(
            UpdatePublicationError::ExistingRecordUnavailable.to_string(),
            "the existing application policy record is unavailable"
        );
        assert_eq!(
            UpdatePublicationError::NewRecordEncodingInvalid.to_string(),
            "the promoted update record cannot be published"
        );
    }
}
