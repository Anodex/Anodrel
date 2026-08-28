//! Closed UI Automation scroll values and routes.

use std::time::Duration;

#[cfg(test)]
use std::sync::Arc;

use anodrel_ui::ElementId;
use anodrel_ui_session::UiDocumentRevision;

use super::{UiAutomationScrollMailbox, UiAutomationScrollSink, Wake};

/// Maximum time an automation caller waits for the owning UI thread.
pub const UI_AUTOMATION_SCROLL_RESPONSE_TIMEOUT: Duration = Duration::from_millis(250);

/// One closed vertical movement command a provider may offer to its host.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAutomationScrollCommand {
    /// Moves by the host's standard local line amount.
    Line {
        /// Whether the movement goes toward the document's end.
        forward: bool,
    },
    /// Moves by the current viewport size.
    Page {
        /// Whether the movement goes toward the document's end.
        forward: bool,
    },
    /// Moves to a vertical percentage in the closed range 0 through 100.
    Percent {
        /// The requested percentage.
        percent: f64,
    },
    /// Reveals one permitted semantic descendant of the selected viewport.
    ///
    /// The owner revalidates both this item and the enclosing viewport against
    /// its current document and layout before it changes host-retained state.
    ScrollIntoView {
        /// The semantic item that should become visible.
        item: ElementId,
    },
}

/// The finite immutable vertical state a provider publishes.
#[derive(Clone, Debug, PartialEq)]
pub struct UiAutomationScrollSnapshot {
    target: ElementId,
    vertical_scroll_percent: f64,
    vertical_view_size: f64,
}

impl UiAutomationScrollSnapshot {
    /// Builds a snapshot for one genuinely overflowing vertical viewport.
    #[must_use]
    pub fn new(
        target: ElementId,
        viewport_height: f32,
        content_height: f32,
        offset_y: f32,
    ) -> Option<Self> {
        let viewport = f64::from(viewport_height);
        let content = f64::from(content_height);
        let offset = f64::from(offset_y);
        if !viewport.is_finite()
            || !content.is_finite()
            || !offset.is_finite()
            || viewport <= 0.0
            || content <= viewport
        {
            return None;
        }
        let maximum = content - viewport;
        Some(Self {
            target,
            vertical_scroll_percent: (offset.clamp(0.0, maximum) / maximum) * 100.0,
            vertical_view_size: ((viewport / content) * 100.0).clamp(0.0, 100.0),
        })
    }

    /// Returns the selected semantic viewport identity.
    #[must_use]
    pub const fn target(&self) -> &ElementId {
        &self.target
    }

    /// Returns the copied vertical scroll percentage.
    #[must_use]
    pub const fn vertical_scroll_percent(&self) -> f64 {
        self.vertical_scroll_percent
    }

    /// Returns the copied vertical view-size percentage.
    #[must_use]
    pub const fn vertical_view_size(&self) -> f64 {
        self.vertical_view_size
    }
}

/// One closed command waiting to be revalidated by its owning UI thread.
#[derive(Clone, Debug)]
pub struct UiAutomationScrollRequest {
    pub(super) id: u64,
    pub(super) revision: Option<UiDocumentRevision>,
    pub(super) target: ElementId,
    pub(super) command: UiAutomationScrollCommand,
}

impl UiAutomationScrollRequest {
    /// Returns the identity used only to complete this exact route entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the provider revision, if this is an authenticated session.
    #[must_use]
    pub const fn revision(&self) -> Option<UiDocumentRevision> {
        self.revision
    }

    /// Returns the selected scroll viewport identity for revalidation.
    #[must_use]
    pub const fn target(&self) -> &ElementId {
        &self.target
    }

    /// Returns the closed host command.
    #[must_use]
    pub fn command(&self) -> UiAutomationScrollCommand {
        self.command.clone()
    }
}

/// A revision-bound route before the host attaches it to a private window.
#[derive(Clone, Debug)]
pub struct UiAutomationScrollRoute {
    pub(super) mailbox: UiAutomationScrollMailbox,
    pub(super) revision: Option<UiDocumentRevision>,
}

impl UiAutomationScrollRoute {
    /// Binds this route to one payload-free host-private wake message.
    #[must_use]
    pub fn for_window(&self, window: isize, wake_message: u32) -> UiAutomationScrollSink {
        UiAutomationScrollSink {
            mailbox: self.mailbox.clone(),
            revision: self.revision,
            wake: Wake::Window {
                window,
                message: wake_message,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn with_notifier(
        &self,
        notifier: impl Fn() -> bool + Send + Sync + 'static,
    ) -> UiAutomationScrollSink {
        UiAutomationScrollSink {
            mailbox: self.mailbox.clone(),
            revision: self.revision,
            wake: Wake::Notifier(Arc::new(notifier)),
        }
    }
}
