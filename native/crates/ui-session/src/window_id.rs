//! Canonical logical identities for session-owned views.

use std::{fmt, num::NonZeroU16};

/// The host never reuses more than this many secondary identities in one
/// authenticated UI session.
pub const MAX_SECONDARY_WINDOW_IDENTITIES: u16 = u16::MAX;

/// A session-scoped logical view identity.
///
/// This value is not a native handle, process identifier, pointer, or global
/// name. A host must resolve it only inside the authenticated session that
/// issued it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UiWindowId {
    /// The one host-associated view every UI session begins with.
    Primary,
    /// A host-issued secondary view identity.
    Secondary(NonZeroU16),
}

impl UiWindowId {
    /// Returns the primary identity, whose protocol spelling is `main`.
    #[must_use]
    pub const fn primary() -> Self {
        Self::Primary
    }

    /// Parses one exact protocol spelling for a logical view identity.
    pub fn parse(value: &str) -> Result<Self, UiWindowIdError> {
        if value == "main" {
            return Ok(Self::Primary);
        }
        let Some(number) = value.strip_prefix("window-") else {
            return Err(UiWindowIdError::Invalid);
        };
        if number.is_empty()
            || number.starts_with('0')
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(UiWindowIdError::Invalid);
        }
        let number = number
            .parse::<u16>()
            .ok()
            .and_then(NonZeroU16::new)
            .ok_or(UiWindowIdError::Invalid)?;
        Ok(Self::Secondary(number))
    }

    /// Returns whether this is the primary session view.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        matches!(self, Self::Primary)
    }

    /// Returns the canonical protocol spelling for this session-scoped identity.
    #[must_use]
    pub fn to_protocol_string(&self) -> String {
        match self {
            Self::Primary => "main".to_owned(),
            Self::Secondary(number) => format!("window-{}", number.get()),
        }
    }
}

impl fmt::Display for UiWindowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_protocol_string())
    }
}

/// A safe category for a rejected logical view identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiWindowIdError {
    /// The value was not `main` or a canonical `window-<n>` value in range.
    Invalid,
}

impl fmt::Display for UiWindowIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UI window identity is invalid")
    }
}

impl std::error::Error for UiWindowIdError {}
