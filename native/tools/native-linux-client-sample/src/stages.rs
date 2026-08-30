//! Closed safe exit stages for the Linux child transport probe.

#[derive(Clone, Copy)]
pub enum Stage {
    Completed,
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    HealthRejected,
}

impl Stage {
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
