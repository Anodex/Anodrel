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
mod state;
mod title;

use std::fmt;

pub use state::{
    WINDOW_STATE_RESPONSE_TIMEOUT, WindowStateMailbox, WindowStateRequest, WindowStateService,
};
pub use title::{WINDOW_TITLE_RESPONSE_TIMEOUT, WindowTitleMailbox, WindowTitleRequest};

/// Maximum UTF-16 code units an application may propose.
///
/// Measured the way `SetWindowTextW` counts, so a proposal that validates here
/// never needs truncating on its way out. Long enough for a document name and
/// short enough that the composed caption stays legible where it is shown.
pub const MAX_PROPOSAL_UTF16_UNITS: usize = 96;

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
/// a target, observe the resulting state, or extend the list with another
/// User32 operation. See `docs/WINDOW_STATE.md` and Decision 0072.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowState {
    /// Put the session's window in the normal Windows minimised state.
    Minimized,
    /// Expand the session's window to the normal Windows maximised state.
    Maximized,
    /// Return the session's window to its normal restored state.
    Restored,
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
mod tests {
    use super::{
        MAX_PROPOSAL_UTF16_UNITS, TITLE_SEPARATOR, WindowCommandError, WindowTitleInputError,
        WindowTitleProposal, compose,
    };

    fn proposal(value: &str) -> WindowTitleProposal {
        WindowTitleProposal::new(value).expect("the test proposal is valid")
    }

    #[test]
    fn accepts_an_ordinary_document_name() {
        let title = proposal("Quarterly Report.pdf");
        assert_eq!(title.as_str(), "Quarterly Report.pdf");
    }

    #[test]
    fn requires_the_proposal_to_carry_text() {
        assert_eq!(
            WindowTitleProposal::new(""),
            Err(WindowTitleInputError::Empty)
        );
    }

    #[test]
    fn measures_length_in_utf16_units_like_the_native_call() {
        // An emoji is one character, four UTF-8 bytes, and two UTF-16 units.
        // Only the last matches what SetWindowTextW counts.
        let emoji = "\u{1F680}";
        assert_eq!(emoji.chars().count(), 1);
        assert_eq!(emoji.len(), 4);
        assert_eq!(emoji.encode_utf16().count(), 2);

        let exact = emoji.repeat(MAX_PROPOSAL_UTF16_UNITS / 2);
        assert!(WindowTitleProposal::new(exact.clone()).is_ok());
        assert_eq!(
            WindowTitleProposal::new(format!("{exact}{emoji}")),
            Err(WindowTitleInputError::TooLarge)
        );
    }

    #[test]
    fn accepts_the_exact_bound_and_rejects_one_unit_more() {
        assert!(WindowTitleProposal::new("t".repeat(MAX_PROPOSAL_UTF16_UNITS)).is_ok());
        assert_eq!(
            WindowTitleProposal::new("t".repeat(MAX_PROPOSAL_UTF16_UNITS + 1)),
            Err(WindowTitleInputError::TooLarge)
        );
    }

    #[test]
    fn rejects_every_control_character_including_a_line_feed() {
        // The line feed is the one that separates this rule from the
        // notification body's. A title is a label on one line, so a newline
        // could move the visible text away from the host's suffix.
        for forged in [
            "Report\nWindows Security",
            "Report\r\u{2014} Windows Security",
            "Report\u{1B}[2K",
            "Report\u{0}",
            "Report\u{85}",
        ] {
            assert_eq!(
                WindowTitleProposal::new(forged),
                Err(WindowTitleInputError::ControlCharacter),
                "{forged:?} was accepted"
            );
        }
    }

    #[test]
    fn the_host_suffix_survives_a_proposal_that_impersonates_another_application() {
        // The whole point of composition. Whatever the proposal claims, the
        // caption still ends with the name the host validated.
        let composed = compose(&proposal("Windows Security"), Some("Anodrel Sample"));
        assert_eq!(composed, "Windows Security \u{2014} Anodrel Sample");
        assert!(composed.ends_with("Anodrel Sample"));
    }

    #[test]
    fn a_proposal_cannot_forge_a_second_suffix_that_outranks_the_real_one() {
        // A proposal may contain the separator - it is an ordinary character -
        // so the caption can read as though it ended earlier than it does. What
        // must hold is that the real name is still last, because that is the
        // part a truncating task switcher drops first and a reader trusts.
        let composed = compose(
            &proposal("Report \u{2014} Some Other App"),
            Some("Anodrel Sample"),
        );
        assert_eq!(
            composed,
            "Report \u{2014} Some Other App \u{2014} Anodrel Sample"
        );
        assert!(composed.ends_with(&format!("{TITLE_SEPARATOR}Anodrel Sample")));
        assert_eq!(composed.matches("Anodrel Sample").count(), 1);
    }

    #[test]
    fn a_session_without_a_validated_name_gets_its_proposal_alone() {
        // An absent claim about which application this is beats an unfounded
        // one, so nothing is appended rather than something unverified.
        assert_eq!(compose(&proposal("Report"), None), "Report");
        assert_eq!(compose(&proposal("Report"), Some("")), "Report");
        assert_eq!(compose(&proposal("Report"), Some("   ")), "Report");
    }

    #[test]
    fn no_failure_message_repeats_the_text_that_was_refused() {
        // An error message is another string that reaches logs and diagnostics.
        // Text refused for being unsafe to display must not travel there.
        let secret = "MarkerZQX";
        let errors = [
            WindowTitleProposal::new(String::new()).unwrap_err(),
            WindowTitleProposal::new(format!("{secret}\n")).unwrap_err(),
            WindowTitleProposal::new(secret.repeat(MAX_PROPOSAL_UTF16_UNITS)).unwrap_err(),
        ];
        for error in errors {
            assert!(
                !error.to_string().contains(secret),
                "{error:?} echoed input"
            );
        }
    }

    #[test]
    fn service_failures_describe_the_request_rather_than_the_host() {
        // Neither category may reveal whether a window exists, which one, or
        // what the operating system said.
        for error in [WindowCommandError::Unavailable, WindowCommandError::Busy] {
            let message = error.to_string();
            for leaked in ["handle", "hwnd", "0x", "error code", "user32"] {
                assert!(
                    !message.to_lowercase().contains(leaked),
                    "{error:?} leaks native detail"
                );
            }
        }
    }
}
