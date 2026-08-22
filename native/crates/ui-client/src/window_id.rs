//! Opaque typed identities for the bounded session-window facade.

use std::{fmt, num::NonZeroU16};

use anodrel_ui_session::UiWindowId;

use crate::UiClientError;

/// One host-issued secondary view identity from this authenticated session.
///
/// This is not a native handle, a process identifier, a desktop-wide name, or
/// a value callers can construct. The typed facade creates it only after the
/// host accepted a secondary view. It is accepted by the two operations whose
/// contract specifically permits an existing secondary target.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SecondaryWindowId(NonZeroU16);

impl SecondaryWindowId {
    /// Encodes this session-local identity for the already-authenticated
    /// protocol request. This stays crate-private so client code cannot turn
    /// arbitrary strings into targetable identities.
    pub(crate) fn protocol_string(self) -> String {
        format!("window-{}", self.0.get())
    }

    pub(crate) fn parse_response(value: &str) -> Result<Self, UiClientError> {
        match UiWindowId::parse(value).map_err(|_| UiClientError::ResponseInvalid)? {
            UiWindowId::Secondary(value) => Ok(Self(value)),
            UiWindowId::Primary => Err(UiClientError::ResponseInvalid),
        }
    }
}

impl fmt::Display for SecondaryWindowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.protocol_string())
    }
}

/// The logical source view carried by a tagged multi-window UI action.
///
/// `Main` is the one host-associated primary view. A secondary value is one
/// that the host issued in this same authenticated session; neither variant
/// carries native window state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SessionWindowId {
    /// The primary session view, spelled `main` in the protocol.
    Main,
    /// One host-issued secondary session view.
    Secondary(SecondaryWindowId),
}

impl SessionWindowId {
    /// Returns whether this is the authenticated session's primary view.
    #[must_use]
    pub const fn is_main(self) -> bool {
        matches!(self, Self::Main)
    }

    pub(crate) fn parse_event(value: &str) -> Result<Self, UiClientError> {
        match UiWindowId::parse(value).map_err(|_| UiClientError::ResponseInvalid)? {
            UiWindowId::Primary => Ok(Self::Main),
            UiWindowId::Secondary(value) => Ok(Self::Secondary(SecondaryWindowId(value))),
        }
    }
}
