//! Closed safe exit stages for the retained Linux Session Lab child.

#[derive(Clone, Copy)]
pub enum Stage {
    BootstrapUnreadable,
    EndpointUnavailable,
    AuthenticationRejected,
    HealthRejected,
}

impl Stage {
    pub const fn code(self) -> u8 {
        match self {
            Self::BootstrapUnreadable => 41,
            Self::EndpointUnavailable => 42,
            Self::AuthenticationRejected => 43,
            Self::HealthRejected => 44,
        }
    }
}
