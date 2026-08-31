//! Fixed machine-policy publication for one promoted Anodrel release.

use std::fmt;

use crate::PromotedRelease;

mod raw;

/// A promoted release whose record was written to the fixed machine policy key.
pub struct PublishedRelease {
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

#[cfg(test)]
mod tests {
    use super::{PublicationError, raw};

    #[test]
    fn publication_uses_the_exact_fixed_machine_policy_location() {
        assert_eq!(
            raw::policy_path("org.anodrel.release-test"),
            "Software\\Anodrel\\Applications\\org.anodrel.release-test"
        );
        assert_eq!(raw::record_value_name(), "record");
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
}
