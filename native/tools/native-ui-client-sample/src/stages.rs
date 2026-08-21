//! Closed, non-secret exit stages for the compiled native UI diagnostic.

/// The fixed diagnostic either completes or stops at one documented boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    DocumentRejected,
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
            Self::BootstrapUnreadable => 41,
            Self::EndpointUnavailable => 42,
            Self::AuthenticationRejected => 43,
            Self::DocumentRejected => 44,
            Self::EventReadFailed => 45,
            Self::ActionNotObserved => 46,
            Self::CloseRejected => 47,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stage;

    const ALL: [Stage; 8] = [
        Stage::Completed,
        Stage::BootstrapUnreadable,
        Stage::EndpointUnavailable,
        Stage::AuthenticationRejected,
        Stage::DocumentRejected,
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
                .all(|code| (41..64).contains(&code))
        );
    }
}
