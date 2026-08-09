#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows delivery of one bounded notification.
//!
//! The adapter owns exactly one notification-area entry for one host window and
//! shows the validated title and body on it. It builds no command line, selects
//! no sound, accepts no application artwork, and returns no native status. See
//! `docs/NOTIFICATIONS.md` and Decision 0062.

mod raw;

use std::{fmt, io};

use anodrel_notifications::{Notification, NotificationService, NotificationServiceError};

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
    window: raw::Handle,
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
        let icon = if icon == 0 { raw::default_icon() } else { icon };
        raw::add_icon(window, icon, ENTRY_TIP).map_err(NotificationSetupError::Io)?;
        Ok(Self { window })
    }
}

impl NotificationService for WindowsNotifications {
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
        raw::show_balloon(
            self.window,
            notification.title().as_str(),
            notification.body().as_str(),
        )
        // Every native failure collapses to one category. A refusal must not
        // distinguish a muted application from a busy shell.
        .map_err(|_| NotificationServiceError::Unavailable)
    }
}

impl Drop for WindowsNotifications {
    fn drop(&mut self) {
        // A stale entry left in the notification area would outlive the surface
        // it belongs to. An already-removed entry makes this harmless.
        let _ = raw::remove_icon(self.window);
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
    /// Windows refused to create the notification-area entry.
    Io(io::Error),
}

impl fmt::Display for NotificationSetupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoWindow => "notifications need a host window",
            Self::Io(_) => "the notification area is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NotificationSetupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NoWindow => None,
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

    #[test]
    fn a_live_entry_never_reveals_its_handle_in_debug_output() {
        let entry = WindowsNotifications { window: -1 };
        assert_eq!(format!("{entry:?}"), "WindowsNotifications(..)");
        // Dropping an entry that was never added is harmless best-effort
        // cleanup, which is what keeps every error path free of a stale icon.
        drop(entry);
    }
}
