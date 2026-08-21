//! Closed, non-secret exit stages for the native development probe.

/// The one fixed development probe either completed or stopped at one boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    HealthRejected,
}

impl Stage {
    /// Returns the probe's stable process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::BootstrapUnreadable => 31,
            Self::EndpointUnavailable => 32,
            Self::AuthenticationRejected => 33,
            Self::HealthRejected => 34,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stage;

    const ALL: [Stage; 5] = [
        Stage::Completed,
        Stage::BootstrapUnreadable,
        Stage::EndpointUnavailable,
        Stage::AuthenticationRejected,
        Stage::HealthRejected,
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
        assert!(codes.into_iter().all(|code| code < 64));
    }
}
