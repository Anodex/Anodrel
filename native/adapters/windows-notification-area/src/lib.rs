#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only ownership of one Windows notification-area entry.
//!
//! This adapter owns the Shell32 entry that a Windows host may use for bounded
//! information balloons now and one future tray surface later. It exposes no
//! application protocol, native handle, callback, icon selection, or process
//! authority. Callers choose only host-owned values on the owning UI thread.
//! See `docs/NOTIFICATION_AREA.md`.

mod raw;

use std::{fmt, io};

/// Maximum UTF-16 code units in a host-owned notification-area tooltip.
///
/// The underlying `NOTIFYICONDATAW` field has one additional unit for its
/// required terminator.
pub const MAX_TOOLTIP_UTF16_UNITS: usize = raw::TIP_UNITS - 1;

/// One host-owned notification-area entry, removed when this value is dropped.
pub struct NotificationArea {
    window: raw::Handle,
}

impl NotificationArea {
    /// Creates one notification-area entry for a live host window.
    ///
    /// `icon` is host-selected artwork; zero chooses Windows' shared
    /// application icon. `tooltip` is host-owned text and must fit exactly;
    /// this boundary never truncates a value before it reaches Shell32.
    ///
    /// Callers must invoke this on the UI thread that owns `window`. The
    /// returned value owns the Shell32 entry until it is dropped.
    pub fn create(
        window: isize,
        icon: isize,
        tooltip: &str,
    ) -> Result<Self, NotificationAreaError> {
        if window == 0 {
            return Err(NotificationAreaError::NoWindow);
        }
        if !tooltip_fits(tooltip) {
            return Err(NotificationAreaError::TooltipInvalid);
        }
        let icon = if icon == 0 { raw::default_icon() } else { icon };
        raw::add_icon(window, icon, tooltip).map_err(NotificationAreaError::Io)?;
        Ok(Self { window })
    }

    /// Displays one silent information balloon on this entry.
    ///
    /// The caller must supply values that already fit the documented fixed
    /// Shell32 fields. The adapter maps every native failure to `io::Error` so
    /// a higher-level capability can choose its own safe public category.
    pub fn show_information(&self, title: &str, body: &str) -> io::Result<()> {
        raw::show_balloon(self.window, title, body)
    }

    /// Enables one private host callback message for this existing entry.
    ///
    /// The value stays entirely on the native side: callers must reserve it
    /// from their own private window-message range, and this adapter exposes
    /// neither callback delivery nor a native handle. A later tray host uses
    /// this only when it needs local interaction; notification-only sessions
    /// never need to configure it.
    pub fn set_callback_message(&self, message: u32) -> Result<(), NotificationAreaError> {
        if message == 0 {
            return Err(NotificationAreaError::CallbackInvalid);
        }
        raw::set_callback_message(self.window, message).map_err(NotificationAreaError::Io)
    }
}

impl Drop for NotificationArea {
    fn drop(&mut self) {
        // A stale entry would outlive its host surface. Shell32 treats an
        // already absent entry as a harmless best-effort cleanup failure.
        let _ = raw::remove_icon(self.window);
    }
}

impl fmt::Debug for NotificationArea {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NotificationArea(..)")
    }
}

/// A safe failure while adding one host-owned notification-area entry.
#[derive(Debug)]
pub enum NotificationAreaError {
    /// The caller supplied no host window to own the entry.
    NoWindow,
    /// The host-owned tooltip does not fit the Shell32 field exactly.
    TooltipInvalid,
    /// The host supplied no private message for local callback routing.
    CallbackInvalid,
    /// Windows refused to create the entry.
    Io(io::Error),
}

impl fmt::Display for NotificationAreaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoWindow => "the notification area needs a host window",
            Self::TooltipInvalid => "the notification-area tooltip is invalid",
            Self::CallbackInvalid => "the notification-area callback is invalid",
            Self::Io(_) => "the notification area is unavailable",
        })
    }
}

impl std::error::Error for NotificationAreaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NoWindow | Self::TooltipInvalid | Self::CallbackInvalid => None,
        }
    }
}

fn tooltip_fits(value: &str) -> bool {
    !value.is_empty()
        && value.encode_utf16().count() <= MAX_TOOLTIP_UTF16_UNITS
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::{MAX_TOOLTIP_UTF16_UNITS, NotificationArea, NotificationAreaError, tooltip_fits};

    #[test]
    fn refuses_to_create_an_entry_without_a_host_window() {
        assert!(matches!(
            NotificationArea::create(0, 0, "Anodrel"),
            Err(NotificationAreaError::NoWindow)
        ));
    }

    #[test]
    fn tooltip_validation_counts_utf16_units_and_refuses_controls() {
        assert!(tooltip_fits(&"a".repeat(MAX_TOOLTIP_UTF16_UNITS)));
        assert!(tooltip_fits(
            &"\u{1F680}".repeat(MAX_TOOLTIP_UTF16_UNITS / 2)
        ));
        assert!(!tooltip_fits(&"a".repeat(MAX_TOOLTIP_UTF16_UNITS + 1)));
        assert!(!tooltip_fits("Anodrel\nTray"));
        assert!(!tooltip_fits(""));
    }

    #[test]
    fn debug_output_hides_native_window_material() {
        let entry = NotificationArea { window: -1 };
        assert_eq!(format!("{entry:?}"), "NotificationArea(..)");
        drop(entry);
    }

    #[test]
    fn refuses_an_absent_private_callback_message() {
        let entry = NotificationArea { window: -1 };
        assert!(matches!(
            entry.set_callback_message(0),
            Err(NotificationAreaError::CallbackInvalid)
        ));
        drop(entry);
    }
}
