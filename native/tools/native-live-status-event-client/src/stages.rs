//! Closed, non-secret exit stages for the live-status event diagnostic.

/// The fixed two-document diagnostic either completes or stops at one boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    InitialDocumentRejected,
    ReplacementDocumentRejected,
    EventReadFailed,
    ActionNotObserved,
    CloseRejected,
}

impl Stage {
    /// Returns the diagnostic's stable process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::BootstrapUnreadable => 61,
            Self::EndpointUnavailable => 62,
            Self::AuthenticationRejected => 63,
            Self::InitialDocumentRejected => 64,
            Self::ReplacementDocumentRejected => 65,
            Self::EventReadFailed => 66,
            Self::ActionNotObserved => 67,
            Self::CloseRejected => 68,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stage;

    const ALL: [Stage; 9] = [
        Stage::Completed,
        Stage::BootstrapUnreadable,
        Stage::EndpointUnavailable,
        Stage::AuthenticationRejected,
        Stage::InitialDocumentRejected,
        Stage::ReplacementDocumentRejected,
        Stage::EventReadFailed,
        Stage::ActionNotObserved,
        Stage::CloseRejected,
    ];

    #[test]
    fn only_completion_reports_success() {
        assert_eq!(Stage::Completed.code(), 0);
        for stage in ALL.into_iter().filter(|stage| *stage != Stage::Completed) {
            assert_ne!(stage.code(), 0, "{stage:?} must fail closed");
        }
    }

    #[test]
    fn every_stage_has_a_distinct_small_code() {
        let mut codes = ALL.map(Stage::code).to_vec();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ALL.len());
        assert!(
            codes
                .into_iter()
                .filter(|code| *code != 0)
                .all(|code| (61..96).contains(&code))
        );
    }
}
