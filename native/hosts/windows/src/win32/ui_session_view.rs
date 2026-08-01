//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use anodrel_canvas::Point;
use anodrel_core::SessionCloseSignal;
use anodrel_ui::UiEvent;
use anodrel_ui_session::{UiDocumentMailbox, UiDocumentRevision, UiInputCandidate, UiInputMailbox};

use super::ui_lab::UiLab;

/// A native session view with no application input or event delivery.
#[derive(Clone)]
pub(super) struct UiSessionView {
    lab: UiLab,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    revision: UiDocumentRevision,
}

impl UiSessionView {
    /// Creates the host-owned waiting surface for one supplied session mailbox.
    pub(super) fn new(
        mailbox: UiDocumentMailbox,
        input_mailbox: UiInputMailbox,
        close_signal: SessionCloseSignal,
    ) -> Self {
        Self {
            lab: UiLab::waiting_for_session(),
            mailbox,
            input_mailbox,
            close_signal,
            revision: UiDocumentRevision::INITIAL,
        }
    }

    /// Applies at most one newer accepted snapshot from this view's mailbox.
    pub(super) fn poll(&mut self) -> (bool, bool) {
        let close_requested = self.close_signal.take();
        let Some(snapshot) = self.mailbox.take() else {
            return (false, close_requested);
        };
        if snapshot.revision() <= self.revision {
            return (false, close_requested);
        }
        self.revision = snapshot.revision();
        self.lab.replace_document(snapshot.document().clone());
        (true, close_requested)
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
    use anodrel_core::SessionCloseSignal;
    use anodrel_ui::UiEvent;
    use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSession, UiInputMailbox};

    use super::UiSessionView;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.root","kind":"text","value":"Connected","fontSize":16,"tone":"primary"}}"#;
    const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.action","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;

    #[test]
    fn applies_only_a_newer_snapshot_from_its_own_mailbox() {
        let mailbox = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(
            mailbox.clone(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
        );
        let mut session = UiDocumentSession::new();
        session
            .replace_document(DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));

        assert_eq!(view.poll(), (true, false));
        assert_eq!(view.poll(), (false, false));
    }

    #[test]
    fn consumes_only_its_supplied_session_close_signal() {
        let signal = SessionCloseSignal::default();
        let mut view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            signal.clone(),
        );

        assert_eq!(view.poll(), (false, false));
        signal.request();
        assert_eq!(view.poll(), (false, true));
        assert_eq!(view.poll(), (false, false));
    }

    #[test]
    fn queues_a_focused_action_only_with_the_current_document_revision() {
        let mailbox = UiDocumentMailbox::new();
        let inputs = UiInputMailbox::new();
        let mut view = UiSessionView::new(
            mailbox.clone(),
            inputs.clone(),
            SessionCloseSignal::default(),
        );
        let mut session = UiDocumentSession::new();
        session
            .replace_document(ACTION_DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));
        assert_eq!(view.poll(), (true, false));

        assert!(view.focus_next(920.0, 660.0));
        assert!(view.activate_focused(920.0, 660.0));
        let batch = inputs.drain();
        assert_eq!(batch.dropped(), 0);
        let candidates = batch.into_candidates();
        assert_eq!(candidates.len(), 1);
        let (revision, UiEvent::ActionInvoked(action)) = candidates
            .into_iter()
            .next()
            .expect("one action candidate exists")
            .into_parts();
        assert_eq!(revision.value(), 1);
        assert_eq!(action.as_str(), "session.action");
    }
}
