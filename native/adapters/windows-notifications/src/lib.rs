#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows delivery of one bounded notification.
//!
//! The adapter owns exactly one notification-area entry for one host window and
//! shows the validated title and body on it. It builds no command line, selects
//! no sound, accepts no application artwork, and returns no native status. See
//! `docs/NOTIFICATIONS.md` and Decision 0062.

use std::fmt;

use anodrel_notifications::{Notification, NotificationService, NotificationServiceError};
use anodrel_windows_notification_area::{NotificationArea, NotificationAreaError};

/// The hover text shown for the host's notification-area entry.
///
/// This is host-owned and fixed. An application cannot supply, read, or change
/// it, so the entry cannot be made to name or impersonate anything.
const ENTRY_TIP: &str = "Anodrel";

/// One host-owned notification-area entry, removed when this value is dropped.
///
/// The entry is added once and kept for the life of this value rather than
/// added and removed around each notification. Removing it immediately would
/// also dismiss the balloon that was just requested, so the entry's lifetime is
/// the session's, not the message's.
pub struct WindowsNotifications {
    area: NotificationArea,
}

impl WindowsNotifications {
    /// Adds this process's notification-area entry for one host-owned window.
    ///
    /// `window` must be a live window owned by the calling host UI thread, and
    /// `icon` is host-selected artwork; passing `0` uses the shared system
    /// application icon. Neither value is ever exposed to an application.
    ///
    /// Call this from the owning UI thread. Shell32 is subject to the same rule
    /// that keeps a pipe worker away from User32.
    pub fn create(window: isize, icon: isize) -> Result<Self, NotificationSetupError> {
        if window == 0 {
            return Err(NotificationSetupError::NoWindow);
        }
        let area = NotificationArea::create(window, icon, ENTRY_TIP).map_err(map_setup_error)?;
        Ok(Self { area })
    }
}

impl NotificationService for WindowsNotifications {
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
        self.area
            .show_information(notification.title().as_str(), notification.body().as_str())
            // Every native failure collapses to one category. A refusal must not
            // distinguish a muted application from a busy shell.
            .map_err(|_| NotificationServiceError::Unavailable)
    }
}

impl fmt::Debug for WindowsNotifications {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The native handle is host-session material and never becomes output.
        formatter.write_str("WindowsNotifications(..)")
    }
}

/// A safe failure category while creating the host notification entry.
#[derive(Debug)]
pub enum NotificationSetupError {
    /// The caller supplied no host window to attach the entry to.
    NoWindow,
    /// The adapter's fixed host tooltip did not fit the Shell32 field rules.
    TooltipInvalid,
    /// Windows refused to create the notification-area entry.
    Io(std::io::Error),
}

fn map_setup_error(error: NotificationAreaError) -> NotificationSetupError {
    match error {
        NotificationAreaError::NoWindow => NotificationSetupError::NoWindow,
        NotificationAreaError::TooltipInvalid => NotificationSetupError::TooltipInvalid,
        NotificationAreaError::Io(error) => NotificationSetupError::Io(error),
    }
}

impl fmt::Display for NotificationSetupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoWindow => "notifications need a host window",
            Self::TooltipInvalid => "notifications need a valid host tooltip",
            Self::Io(_) => "the notification area is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NotificationSetupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NoWindow | Self::TooltipInvalid => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ENTRY_TIP, NotificationSetupError, WindowsNotifications};

    #[test]
    fn refuses_to_create_an_entry_without_a_host_window() {
        assert!(matches!(
            WindowsNotifications::create(0, 0),
            Err(NotificationSetupError::NoWindow)
        ));
    }

    #[test]
    fn setup_failures_state_a_boundary_without_native_detail() {
        assert_eq!(
            NotificationSetupError::NoWindow.to_string(),
            "notifications need a host window"
        );
    }

    #[test]
    fn the_entry_tip_is_fixed_host_text() {
        // An application cannot supply or read this, so the entry cannot be
        // made to name or impersonate anything.
        assert_eq!(ENTRY_TIP, "Anodrel");
    }
}
