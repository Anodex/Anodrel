//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use anodrel_ui_session::{UiDocumentMailbox, UiDocumentRevision};

use super::ui_lab::UiLab;

/// A native session view with no application input or event delivery.
#[derive(Clone)]
pub(super) struct UiSessionView {
    lab: UiLab,
    mailbox: UiDocumentMailbox,
    revision: UiDocumentRevision,
}

impl UiSessionView {
    /// Creates the host-owned waiting surface for one supplied session mailbox.
    pub(super) fn new(mailbox: UiDocumentMailbox) -> Self {
        Self {
            lab: UiLab::waiting_for_session(),
            mailbox,
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

    /// Returns the local native renderer state.
    pub(super) const fn lab(&self) -> &UiLab {
        &self.lab
    }
}

#[cfg(test)]
mod tests {
    use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSession};

    use super::UiSessionView;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.root","kind":"text","value":"Connected","fontSize":16,"tone":"primary"}}"#;

    #[test]
    fn applies_only_a_newer_snapshot_from_its_own_mailbox() {
        let mailbox = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(mailbox.clone());
        let mut session = UiDocumentSession::new();
        session
            .replace_document(DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));

        assert!(view.poll());
        assert!(!view.poll());
    }
}
