//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use anodrel_canvas::Point;
use anodrel_ui::UiEvent;
use anodrel_ui_session::{UiDocumentMailbox, UiDocumentRevision, UiInputCandidate, UiInputMailbox};

use super::ui_lab::UiLab;

/// A native session view with no application input or event delivery.
#[derive(Clone)]
pub(super) struct UiSessionView {
    lab: UiLab,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    revision: UiDocumentRevision,
}

impl UiSessionView {
    /// Creates the host-owned waiting surface for one supplied session mailbox.
    pub(super) fn new(mailbox: UiDocumentMailbox, input_mailbox: UiInputMailbox) -> Self {
        Self {
            lab: UiLab::waiting_for_session(),
            mailbox,
            input_mailbox,
            revision: UiDocumentRevision::INITIAL,
        }
    }

    /// Applies at most one newer accepted snapshot from this view's mailbox.
    pub(super) fn poll(&mut self) -> bool {
        let Some(snapshot) = self.mailbox.take() else {
            return false;
        };
        if snapshot.revision() <= self.revision {
            return false;
        }
        self.revision = snapshot.revision();
        self.lab.replace_document(snapshot.document().clone());
        true
    }

    /// Updates hover state through this view's current native layout.
    pub(super) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.update_hover(width, height, at)
    }

    /// Clears hover state when the native pointer leaves this view.
    pub(super) fn clear_hover(&mut self) -> bool {
        self.lab.clear_hover()
    }

    /// Moves focus through this view's current visible actions.
    pub(super) fn focus_next(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_next(width, height)
    }

    /// Moves focus backwards through this view's current visible actions.
    pub(super) fn focus_previous(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_previous(width, height)
    }

    /// Queues one current pointer-derived semantic action candidate.
    pub(super) fn invoke(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(event) = self.lab.event_at(width, height, at) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Queues one current focused semantic action candidate.
    pub(super) fn activate_focused(&mut self, width: f32, height: f32) -> bool {
        let Some(event) = self.lab.focused_event(width, height) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Returns whether the current layout has a hovered action.
    pub(super) fn is_hovered(&self) -> bool {
        self.lab.hovered.is_some()
    }

    /// Returns the local native renderer state.
    pub(super) const fn lab(&self) -> &UiLab {
        &self.lab
    }

    fn queue_event(&self, event: UiEvent) -> bool {
        if self.revision == UiDocumentRevision::INITIAL {
            return false;
        }
        self.input_mailbox
            .push(UiInputCandidate::new(self.revision, event));
        true
    }
}

#[cfg(test)]
mod tests {
    use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSession, UiInputMailbox};

    use super::UiSessionView;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.root","kind":"text","value":"Connected","fontSize":16,"tone":"primary"}}"#;

    #[test]
    fn applies_only_a_newer_snapshot_from_its_own_mailbox() {
        let mailbox = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(mailbox.clone(), UiInputMailbox::new());
        let mut session = UiDocumentSession::new();
        session
            .replace_document(DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));

        assert!(view.poll());
        assert!(!view.poll());
    }
}
