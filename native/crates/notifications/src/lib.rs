//! Bounded, portable notification values.
//!
//! This crate owns no operating-system call and no native handle. Native
//! adapters use these values to keep their public boundary validated, bounded,
//! and free of anything that could forge a message or a source.
//!
//! A notification is a one-way announce: a title, a body, and nothing else. It
//! has no identifier, replace, revoke, action, callback, or read surface, so it
//! carries no return path back to the application that sent it. See
//! `docs/NOTIFICATIONS.md` and Decision 0062.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

/// Maximum UTF-16 code units in a notification title.
///
/// The first operating-system mapping writes into a fixed 64-unit buffer that
/// includes its terminator, so a title that validates here always fits without
/// truncation. See `docs/NOTIFICATIONS.md`.
pub const MAX_TITLE_UTF16_UNITS: usize = 63;

/// Maximum UTF-16 code units in a notification body.
///
/// The matching operating-system buffer is 256 units including its terminator.
pub const MAX_BODY_UTF16_UNITS: usize = 255;

/// One validated notification title.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationTitle(String);

impl NotificationTitle {
    /// Builds a title after enforcing the documented bounds and character rules.
    ///
    /// A title carries no line breaks at all: the target surface renders it as a
    /// single line, and allowing breaks would let one notification present
    /// itself as more than one message.
    pub fn new(value: impl Into<String>) -> Result<Self, NotificationInputError> {
        let value = value.into();
        validate(&value, MAX_TITLE_UTF16_UNITS, false)?;
        Ok(Self(value))
    }

    /// Returns the validated title text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One validated notification body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationBody(String);

impl NotificationBody {
    /// Builds a body after enforcing the documented bounds and character rules.
    ///
    /// Line feeds are permitted because the target surface renders them as line
    /// breaks. Every other control character is rejected.
    pub fn new(value: impl Into<String>) -> Result<Self, NotificationInputError> {
        let value = value.into();
        validate(&value, MAX_BODY_UTF16_UNITS, true)?;
        Ok(Self(value))
    }

    /// Returns the validated body text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One complete notification ready to hand to a native adapter.
///
/// This is the entire message. There is deliberately no identifier, urgency,
/// category, icon, sound, timeout, or action field: adding one would change the
/// contract in `docs/NOTIFICATIONS.md` and needs its own decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    title: NotificationTitle,
    body: NotificationBody,
}

impl Notification {
    /// Pairs a validated title and body.
    #[must_use]
    pub const fn new(title: NotificationTitle, body: NotificationBody) -> Self {
        Self { title, body }
    }

    /// Returns the validated title.
    #[must_use]
    pub const fn title(&self) -> &NotificationTitle {
        &self.title
    }

    /// Returns the validated body.
    #[must_use]
    pub const fn body(&self) -> &NotificationBody {
        &self.body
    }
}

/// The portable service boundary used by a host core.
///
/// Implementations own the operating-system call. They must not retain a native
/// handle, expose a native status, or log notification text.
///
/// The result reports only that the host handed the values over. It must never
/// describe what the user experienced: whether notifications are silenced,
/// whether a focus mode is active, or whether this application is muted is the
/// user's business and not observable through this boundary.
pub trait NotificationService: fmt::Debug + Send {
    /// Hands one validated notification to the operating system.
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError>;
}

/// A safe failure category returned by a portable notification service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationServiceError {
    /// No surface is available, or the operating system refused.
    ///
    /// This deliberately does not distinguish a muted application from a busy
    /// shell.
    Unavailable,
    /// Another notification for this session is already pending.
    Busy,
}

impl fmt::Display for NotificationServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => "notifications are unavailable",
            Self::Busy => "a notification is already pending",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NotificationServiceError {}

/// A stable validation failure raised before any native call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationInputError {
    /// The value has no text.
    Empty,
    /// The value exceeds its fixed UTF-16 bound.
    TooLarge,
    /// The value contains a control character that is not permitted here.
    ControlCharacter,
}

impl fmt::Display for NotificationInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "notification text is empty",
            Self::TooLarge => "notification text exceeds its fixed size limit",
            Self::ControlCharacter => "notification text contains a control character",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NotificationInputError {}

/// Applies the shared bounds and character rules for one notification value.
///
/// Length is measured in UTF-16 code units so it matches what the native buffer
/// counts. Counting UTF-8 bytes or characters would let a value pass here and
/// still need truncating on its way to the operating system.
fn validate(
    value: &str,
    maximum_units: usize,
    allow_line_feed: bool,
) -> Result<(), NotificationInputError> {
    if value.is_empty() {
        return Err(NotificationInputError::Empty);
    }
    if value
        .chars()
        .any(|character| character.is_control() && !(allow_line_feed && character == '\n'))
    {
        return Err(NotificationInputError::ControlCharacter);
    }
    // `chars().count()` would undercount anything outside the basic plane, and
    // `len()` would overcount most non-ASCII text.
    if value.encode_utf16().count() > maximum_units {
        return Err(NotificationInputError::TooLarge);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_BODY_UTF16_UNITS, MAX_TITLE_UTF16_UNITS, Notification, NotificationBody,
        NotificationInputError, NotificationServiceError, NotificationTitle,
    };

    #[test]
    fn accepts_ordinary_text_inside_the_bounds() {
        let title = NotificationTitle::new("Build finished").expect("the title is valid");
        let body = NotificationBody::new("Two targets\nzero warnings").expect("the body is valid");
        let notification = Notification::new(title, body);

        assert_eq!(notification.title().as_str(), "Build finished");
        assert_eq!(notification.body().as_str(), "Two targets\nzero warnings");
    }

    #[test]
    fn requires_both_values_to_carry_text() {
        assert_eq!(
            NotificationTitle::new(""),
            Err(NotificationInputError::Empty)
        );
        assert_eq!(
            NotificationBody::new(""),
            Err(NotificationInputError::Empty)
        );
    }

    #[test]
    fn measures_length_in_utf16_units_like_the_native_buffer() {
        // An emoji is one character, four UTF-8 bytes, and two UTF-16 units.
        // Only the last measure matches what the native buffer counts, so it is
        // the one that decides.
        let emoji = "\u{1F680}";
        assert_eq!(emoji.chars().count(), 1);
        assert_eq!(emoji.len(), 4);
        assert_eq!(emoji.encode_utf16().count(), 2);

        let exact = emoji.repeat(MAX_TITLE_UTF16_UNITS / 2);
        assert!(NotificationTitle::new(exact.clone()).is_ok());
        assert_eq!(
            NotificationTitle::new(format!("{exact}{emoji}")),
            Err(NotificationInputError::TooLarge)
        );
    }

    #[test]
    fn accepts_each_value_exactly_at_its_bound_and_rejects_one_unit_more() {
        assert!(NotificationTitle::new("t".repeat(MAX_TITLE_UTF16_UNITS)).is_ok());
        assert_eq!(
            NotificationTitle::new("t".repeat(MAX_TITLE_UTF16_UNITS + 1)),
            Err(NotificationInputError::TooLarge)
        );
        assert!(NotificationBody::new("b".repeat(MAX_BODY_UTF16_UNITS)).is_ok());
        assert_eq!(
            NotificationBody::new("b".repeat(MAX_BODY_UTF16_UNITS + 1)),
            Err(NotificationInputError::TooLarge)
        );
    }

    #[test]
    fn rejects_control_characters_that_could_forge_a_second_message() {
        // A carriage return or an escape could present one notification as more
        // than one message, or misrepresent where it came from.
        for forged in ["Build\rFAILED", "Build\u{1B}[31m", "Build\u{0}done"] {
            assert_eq!(
                NotificationTitle::new(forged),
                Err(NotificationInputError::ControlCharacter)
            );
            assert_eq!(
                NotificationBody::new(forged),
                Err(NotificationInputError::ControlCharacter)
            );
        }
    }

    #[test]
    fn allows_a_line_feed_only_in_the_body() {
        assert!(NotificationBody::new("first\nsecond").is_ok());
        assert_eq!(
            NotificationTitle::new("first\nsecond"),
            Err(NotificationInputError::ControlCharacter)
        );
    }

    #[test]
    fn failure_categories_describe_the_host_rather_than_the_user() {
        // Neither category may hint at whether the user silenced, muted, or
        // ignored this application.
        for error in [
            NotificationServiceError::Unavailable,
            NotificationServiceError::Busy,
        ] {
            let message = error.to_string();
            for leaked in ["mute", "silenc", "focus", "ignor", "dismiss"] {
                assert!(!message.contains(leaked), "{error:?} describes the user");
            }
        }
    }
}
