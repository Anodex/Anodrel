//! Closed errors that never disclose desktop-session or compositor details.

use std::fmt;

/// Closed outcome from the development-only Linux desktop diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxWaylandError {
    /// The inherited desktop session is unavailable or unsuitable.
    DesktopUnavailable,
    /// The compositor lacks a required stable Wayland capability.
    RequiredSupportUnavailable,
    /// The compositor stream violated the narrow protocol contract.
    ProtocolRejected,
    /// Both fixed shared-memory buffers remain owned by the compositor.
    Backpressured,
    /// A caller tried to present a canvas other than the fixed lab size.
    CanvasSizeMismatch,
}

impl fmt::Display for LinuxWaylandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DesktopUnavailable => formatter.write_str("Linux desktop session is unavailable"),
            Self::RequiredSupportUnavailable => {
                formatter.write_str("Linux compositor lacks required Wayland support")
            }
            Self::ProtocolRejected => formatter.write_str("Linux compositor protocol was rejected"),
            Self::Backpressured => {
                formatter.write_str("Linux compositor has not released a frame buffer")
            }
            Self::CanvasSizeMismatch => {
                formatter.write_str("Linux lab canvas has an unexpected size")
            }
        }
    }
}

impl std::error::Error for LinuxWaylandError {}
