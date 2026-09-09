//! Bounded, portable window-title values.
//!
//! This crate owns no operating-system call and no native handle. It defines
//! what an application may propose for its window's title, and what the host
//! actually displays — which are deliberately not the same string.
//!
//! An application proposes; the host composes. A title appears in the task
//! switcher, the taskbar, window lists, screen-reader announcements, and
//! screenshots, which are the places a person looks to decide what they are
//! talking to. So the host appends the application's validated display name
//! after validation, where a proposal can neither suppress nor forge it. See
//! `docs/WINDOW_TITLE.md` and Decision 0066.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bridge;
mod focus;
mod fullscreen;
mod size;
mod state;
mod state_changes;
mod state_read;
mod title;

use std::fmt;

pub use focus::{
    WINDOW_FOCUS_RESPONSE_TIMEOUT, WindowFocusMailbox, WindowFocusRequest, WindowFocusService,
};
pub use fullscreen::{
    WINDOW_FULLSCREEN_RESPONSE_TIMEOUT, WindowFullscreenMailbox, WindowFullscreenRequest,
    WindowFullscreenService,
};
pub use size::{
    WINDOW_SIZE_RESPONSE_TIMEOUT, WindowSizeMailbox, WindowSizeRequest, WindowSizeService,
};
pub use state::{
    WINDOW_STATE_RESPONSE_TIMEOUT, WindowStateMailbox, WindowStateRequest, WindowStateService,
};
pub use state_changes::{
    WindowStateChangesMailbox, WindowStateChangesService, WindowStateChangesServiceError,
};
pub use state_read::{
    WINDOW_STATE_READ_RESPONSE_TIMEOUT, WindowStateReadMailbox, WindowStateReadRequest,
    WindowStateReadService,
};
pub use title::{WINDOW_TITLE_RESPONSE_TIMEOUT, WindowTitleMailbox, WindowTitleRequest};

/// Maximum UTF-16 code units an application may propose.
///
/// Measured the way `SetWindowTextW` counts, so a proposal that validates here
/// never needs truncating on its way out. Long enough for a document name and
/// short enough that the composed caption stays legible where it is shown.
pub const MAX_PROPOSAL_UTF16_UNITS: usize = 96;

/// Smallest logical client width the public session-window size request accepts.
pub const MIN_WINDOW_CLIENT_WIDTH: u32 = 320;
/// Largest logical client width the public session-window size request accepts.
pub const MAX_WINDOW_CLIENT_WIDTH: u32 = 3_840;
/// Smallest logical client height the public session-window size request accepts.
pub const MIN_WINDOW_CLIENT_HEIGHT: u32 = 240;
/// Largest logical client height the public session-window size request accepts.
pub const MAX_WINDOW_CLIENT_HEIGHT: u32 = 2_160;

/// Separator between the application's proposal and the host's suffix.
///
/// An em dash with surrounding spaces, which is the convention Windows itself
/// uses for `Document — Application`.
pub const TITLE_SEPARATOR: &str = " \u{2014} ";

/// One validated window-title proposal from an application.
///
/// Holding one of these means the bounds and character rules have already been
/// enforced. It is not what gets displayed: see [`compose`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowTitleProposal(String);

impl WindowTitleProposal {
    /// Builds a proposal after enforcing the documented bounds and rules.
    ///
    /// # Errors
    ///
    /// Returns [`WindowTitleInputError`] for an empty value, one longer than
    /// [`MAX_PROPOSAL_UTF16_UNITS`], or one containing any control character.
    pub fn new(value: impl Into<String>) -> Result<Self, WindowTitleInputError> {
        let value = value.into();
        if value.is_empty() {
            return Err(WindowTitleInputError::Empty);
        }
        // Every control character, with no exception for a line feed. A title
        // is a label rendered on one line; a newline could split one window's
        // title into what reads as two, or push the visible text away from the
        // host's suffix. That is the impersonation `compose` prevents, arriving
        // through the character set instead of through the string.
        if value.chars().any(char::is_control) {
            return Err(WindowTitleInputError::ControlCharacter);
        }
        // `chars().count()` would undercount anything outside the basic plane
        // and `len()` would overcount most non-ASCII text. Only UTF-16 units
        // match what the native call counts.
        if value.encode_utf16().count() > MAX_PROPOSAL_UTF16_UNITS {
            return Err(WindowTitleInputError::TooLarge);
        }
        Ok(Self(value))
    }

    /// Returns the validated proposal text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Composes the caption the host will actually display.
///
/// `display_name` must come from the machine-validated installed record, never
/// from the request, the package content, or anything the application can
/// influence at run time. It is appended *after* the proposal has been
/// validated, so no proposal can suppress, duplicate, or forge it.
///
/// A session with no validated display name gets its proposal alone. An absent
/// claim about which application this is beats an unfounded one.
#[must_use]
pub fn compose(proposal: &WindowTitleProposal, display_name: Option<&str>) -> String {
    match display_name.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("{}{TITLE_SEPARATOR}{name}", proposal.as_str()),
        None => proposal.as_str().to_owned(),
    }
}

/// The portable service boundary used by a host core.
///
/// Implementations own the operating-system call. They must not retain a native
/// handle, expose a native status, or accept a window target: the window is
/// resolved from the authenticated session, never named by the caller.
pub trait WindowTitleService: fmt::Debug + Send {
    /// Applies one validated proposal to the session's own window.
    ///
    /// # Errors
    ///
    /// Returns a [`WindowTitleServiceError`] category. Success means the host
    /// applied a title; it deliberately does not report what the composed
    /// caption became.
    fn set_title(&self, proposal: &WindowTitleProposal) -> Result<(), WindowTitleServiceError>;
}

/// A portable request to change the session's own window presentation state.
///
/// This is a closed set rather than a native command or style value. The host
/// resolves the window from the authenticated session; applications cannot name
/// a target or extend the list with another User32 operation. A separately
/// granted pull-only state observation is defined in `docs/WINDOW_STATE_OBSERVATION.md`.
/// See `docs/WINDOW_STATE.md`, Decision 0072, and Decision 0117.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowState {
    /// Put the session's window in the normal Windows minimised state.
    Minimized,
    /// Expand the session's window to the normal Windows maximised state.
    Maximized,
    /// Return the session's window to its normal restored state.
    Restored,
}

/// A closed presentation mode for the session's own window.
///
/// The host resolves the window from the authenticated session and retains all
/// native presentation facts privately. This is neither exclusive display
/// control nor a way to select a monitor, change geometry, or observe current
/// window state. See `docs/WINDOW_FULLSCREEN.md` and Decision 0086.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowFullscreenMode {
    /// Apply reversible borderless fullscreen to the session's own window.
    Fullscreen,
    /// Restore the host-retained normal framed-window presentation.
    Windowed,
}

/// One validated logical client area for the session's own window.
///
/// The dimensions use 96-DPI logical pixels. The native host alone maps them
/// to a framed physical rectangle for the known session window; this value has
/// no position, monitor, native frame, or protocol serialization of its own.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowSize {
    width: u32,
    height: u32,
}

impl WindowSize {
    /// Validates one bounded logical client area.
    ///
    /// # Errors
    ///
    /// Returns [`WindowSizeInputError`] when either dimension is outside the
    /// documented inclusive bound.
    pub const fn new(width: u32, height: u32) -> Result<Self, WindowSizeInputError> {
        if width < MIN_WINDOW_CLIENT_WIDTH || width > MAX_WINDOW_CLIENT_WIDTH {
            return Err(WindowSizeInputError::WidthOutOfRange);
        }
        if height < MIN_WINDOW_CLIENT_HEIGHT || height > MAX_WINDOW_CLIENT_HEIGHT {
            return Err(WindowSizeInputError::HeightOutOfRange);
        }
        Ok(Self { width, height })
    }

    /// Returns the bounded logical client width.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Returns the bounded logical client height.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// A safe failure category returned by a session-owned window command.
///
/// Both title and state commands use these categories. They deliberately do
/// not say whether the session lacks a window or a native call failed, because
/// that distinction would reveal host state the application does not need.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCommandError {
    /// This session has no host window, its command expired, or the native call
    /// failed.
    ///
    /// Deliberately does not distinguish the two: which one it is describes
    /// host state an application has no business learning.
    Unavailable,
    /// Another proposal for this session is still pending.
    Busy,
}

impl fmt::Display for WindowCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => "no session window is available",
            Self::Busy => "a window command is already pending",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowCommandError {}

/// Failure returned by a portable window-title service.
///
/// This alias preserves title-specific source compatibility while sharing the
/// common session-window failure categories with [`WindowStateService`].
pub type WindowTitleServiceError = WindowCommandError;

/// Failure returned by a portable window-state service.
///
/// State commands use the same unavailable and busy meanings as title
/// commands; their protocol mapping remains the shared `window.*` codes.
pub type WindowStateServiceError = WindowCommandError;

/// Failure returned by a portable pull-only window-state observation service.
///
/// Observation has the same unavailable and busy meanings as session-window
/// commands. The common categories intentionally reveal no native failure
/// reason or host topology.
pub type WindowStateReadServiceError = WindowCommandError;

/// Errors shared by the focused session-window service and its host bridge.
pub type WindowFocusServiceError = WindowCommandError;

/// Errors shared by the session fullscreen service and its host bridge.
pub type WindowFullscreenServiceError = WindowCommandError;

/// Errors shared by the session-window size service and its host bridge.
pub type WindowSizeServiceError = WindowCommandError;

/// A stable validation failure raised before any native call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowTitleInputError {
    /// The proposal has no text.
    Empty,
    /// The proposal exceeds [`MAX_PROPOSAL_UTF16_UNITS`].
    TooLarge,
    /// The proposal contains a control character.
    ControlCharacter,
}

/// A stable validation failure raised before any native size request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowSizeInputError {
    /// The requested logical client width was outside its inclusive bound.
    WidthOutOfRange,
    /// The requested logical client height was outside its inclusive bound.
    HeightOutOfRange,
}

impl fmt::Display for WindowSizeInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::WidthOutOfRange => "window client width is outside its fixed limit",
            Self::HeightOutOfRange => "window client height is outside its fixed limit",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowSizeInputError {}

impl fmt::Display for WindowTitleInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // None of these repeats the offending text. A refused value must not be
        // smuggled into a log or a diagnostic by way of its error message.
        let message = match self {
            Self::Empty => "window title is empty",
            Self::TooLarge => "window title exceeds its fixed size limit",
            Self::ControlCharacter => "window title contains a control character",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowTitleInputError {}

#[cfg(test)]
mod tests;
