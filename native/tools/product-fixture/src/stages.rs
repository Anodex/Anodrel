//! The fixture's closed set of safe exit stages.
//!
//! The bootstrap launcher sends child output to `NUL`, so the exit code is the
//! only channel back to a developer. Each stage names a boundary and nothing
//! else: no path, invitation, token, certificate value, or Windows error can
//! reach a host, a log, or a terminal through this type.

/// One boundary the fixture either passed or stopped at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    HostNotReady,
    DocumentRejected,
    EventReadFailed,
    ActionNotObserved,
    CloseRejected,
}

impl Stage {
    /// Returns the documented process exit code for this stage.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Completed => 0,
            Self::BootstrapUnreadable => 11,
            Self::EndpointUnavailable => 12,
            Self::AuthenticationRejected => 13,
            Self::HostNotReady => 14,
            Self::DocumentRejected => 15,
            Self::EventReadFailed => 16,
            Self::ActionNotObserved => 17,
            Self::CloseRejected => 18,
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
        Stage::HostNotReady,
        Stage::DocumentRejected,
        Stage::EventReadFailed,
        Stage::ActionNotObserved,
        Stage::CloseRejected,
    ];

    #[test]
    fn only_a_complete_round_trip_exits_successfully() {
        assert_eq!(Stage::Completed.code(), 0);
        for stage in ALL.into_iter().filter(|stage| *stage != Stage::Completed) {
            assert_ne!(stage.code(), 0, "{stage:?} must not report success");
        }
    }

    #[test]
    fn every_stage_has_a_distinct_documented_code() {
        // `docs/PRODUCT_FIXTURE.md` publishes these codes; a collision would
        // send a developer to the wrong boundary.
        let mut codes = ALL.map(Stage::code).to_vec();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), ALL.len());
    }

    #[test]
    fn stage_codes_stay_clear_of_the_host_shutdown_code() {
        // `anodrel-windows-launch` terminates a child with 0xA11D during host
        // shutdown. A fixture stage must never be mistaken for that.
        for stage in ALL {
            assert!(stage.code() < 32);
        }
    }
}
