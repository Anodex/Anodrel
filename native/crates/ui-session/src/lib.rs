//! Revision-bound state for one current Anodrel UI document.
//!
//! The crate atomically validates and replaces a strict UI document, exposes
//! its monotonic revision, and validates semantic actions against that revision.
//! It has no application identity, transport, renderer, native host, package,
//! callback, protocol operation, or operating-system authority. Its bounded
//! input mailbox stores only host-derived document and native-menu semantic
//! candidates in one ordered bounded queue.
//!
//! See `docs/UI_SESSIONS.md` and Decision 0030 for the complete contract.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod event;
mod field_mailbox;
mod fields;
mod input_mailbox;
mod interaction;
mod mailbox;
mod revision;
mod session;
mod snapshot;

pub use error::UiSessionError;
pub use event::UiApplicationEvent;
pub use field_mailbox::{UI_FIELD_RESPONSE_TIMEOUT, UiFieldMailbox, UiFieldRequest};
pub use fields::{
    MAX_SNAPSHOT_FIELDS, UiFieldReadError, UiFieldReader, UiFieldSnapshot, UiFieldSnapshotError,
    UiFieldValue,
};
pub use input_mailbox::{UI_INPUT_QUEUE_CAPACITY, UiInputBatch, UiInputMailbox};
pub use interaction::{MenuInputCandidate, SessionInteractionCandidate, UiInputCandidate};
pub use mailbox::UiDocumentMailbox;
pub use revision::UiDocumentRevision;
pub use session::UiDocumentSession;
pub use snapshot::UiDocumentSnapshot;

#[cfg(test)]
mod tests {
    use anodrel_ui::{ElementId, UiEvent};
    use anodrel_ui_document::UiDocumentError;

    use super::{
        UI_INPUT_QUEUE_CAPACITY, UiDocumentMailbox, UiDocumentRevision, UiDocumentSession,
        UiInputCandidate, UiInputMailbox, UiSessionError,
    };

    fn action_document(enabled: bool) -> String {
        format!(
            r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"stack","axis":"vertical","padding":{{"left":0,"top":0,"right":0,"bottom":0}},"gap":0,"surfaceTone":"plain","children":[{{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":{enabled},"tone":"accent"}}]}}}}"#
        )
    }

    fn text_document() -> &'static str {
        r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"No action","fontSize":16,"tone":"primary"}}"#
    }

    fn scroll_document() -> &'static str {
        r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#
    }

    fn event() -> UiEvent {
        UiEvent::ActionInvoked(ElementId::new("continue").expect("test ID is valid"))
    }

    #[test]
    fn replaces_a_document_atomically_and_returns_monotonic_revisions() {
        let mut session = UiDocumentSession::new();
        assert_eq!(session.document(), None);

        let first = session
            .replace_document(&action_document(true))
            .expect("document is valid");
        assert_eq!(first.value(), 1);
        assert_eq!(
            session.document().map(|(_, revision)| revision),
            Some(first)
        );

        let second = session
            .replace_document(text_document())
            .expect("document is valid");
        assert_eq!(second.value(), 2);
        assert_eq!(
            session.document().map(|(_, revision)| revision),
            Some(second)
        );
    }

    #[test]
    fn invalid_replacement_preserves_the_current_document_and_revision() {
        let mut session = UiDocumentSession::new();
        let revision = session
            .replace_document(&action_document(true))
            .expect("document is valid");

        assert_eq!(
            session.replace_document("not JSON"),
            Err(UiSessionError::InvalidDocument(
                UiDocumentError::InvalidJson
            ))
        );
        assert_eq!(
            session.document().map(|(_, current)| current),
            Some(revision)
        );
    }

    #[test]
    fn accepts_scroll_documents_only_through_the_explicit_version_two_method() {
        let mut session = UiDocumentSession::new();

        assert_eq!(
            session.replace_document(scroll_document()),
            Err(UiSessionError::InvalidDocument(
                UiDocumentError::UnsupportedFormat
            ))
        );
        assert_eq!(session.document(), None);
        assert_eq!(
            session
                .replace_document_v2(scroll_document())
                .expect("version two document is valid")
                .value(),
            1
        );
    }

    #[test]
    fn clear_invalidates_the_prior_document_without_advancing_when_already_empty() {
        let mut session = UiDocumentSession::new();
        assert_eq!(session.clear_document(), Ok(None));
        let first = session
            .replace_document(&action_document(true))
            .expect("document is valid");
        assert_eq!(first.value(), 1);
        assert_eq!(
            session.clear_document(),
            Ok(Some(
                UiDocumentRevision::default()
                    .next()
                    .unwrap()
                    .next()
                    .unwrap()
            ))
        );
        assert_eq!(session.document(), None);
        assert_eq!(session.clear_document(), Ok(None));
    }

    #[test]
    fn rejects_stale_missing_and_unavailable_actions() {
        let mut session = UiDocumentSession::new();
        let first = session
            .replace_document(&action_document(true))
            .expect("document is valid");
        let second = session
            .replace_document(text_document())
            .expect("document is valid");
        assert_ne!(first, second);
        assert_eq!(
            session.accept_event(first, event()),
            Err(UiSessionError::StaleRevision)
        );
        assert_eq!(
            session.accept_event(second, event()),
            Err(UiSessionError::ActionUnavailable)
        );
        session
            .clear_document()
            .expect("clear can advance revision");
        assert_eq!(
            session.accept_event(second, event()),
            Err(UiSessionError::StaleRevision)
        );
    }

    #[test]
    fn returns_only_the_current_enabled_semantic_action() {
        let mut session = UiDocumentSession::new();
        let revision = session
            .replace_document(&action_document(true))
            .expect("document is valid");
        let accepted = session
            .accept_event(revision, event())
            .expect("event is current and enabled");
        assert_eq!(accepted.revision(), revision);
        assert_eq!(accepted.action().as_str(), "continue");

        let disabled = session
            .replace_document(&action_document(false))
            .expect("document is valid");
        assert_eq!(
            session.accept_event(disabled, event()),
            Err(UiSessionError::ActionUnavailable)
        );
    }

    #[test]
    fn mailbox_coalesces_to_the_newest_pending_snapshot() {
        let mut session = UiDocumentSession::new();
        let first = session
            .replace_document(&action_document(true))
            .expect("first document is valid");
        let first_snapshot = session.snapshot().expect("first snapshot is available");
        let second = session
            .replace_document(text_document())
            .expect("second document is valid");
        let second_snapshot = session.snapshot().expect("second snapshot is available");
        let mailbox = UiDocumentMailbox::new();

        mailbox.publish(second_snapshot);
        mailbox.publish(first_snapshot);
        let snapshot = mailbox.take().expect("newest snapshot is retained");
        assert_eq!(snapshot.revision(), second);
        assert_ne!(snapshot.revision(), first);
        assert_eq!(snapshot.document().root().id().as_str(), "root");
        assert!(mailbox.take().is_none());
    }

    #[test]
    fn input_mailbox_bounds_candidates_and_reports_overflow() {
        let mailbox = UiInputMailbox::new();
        let revision = UiDocumentRevision::INITIAL
            .next()
            .expect("first revision exists");
        for _ in 0..UI_INPUT_QUEUE_CAPACITY + 2 {
            mailbox.push(UiInputCandidate::new(revision, event()));
        }

        let batch = mailbox.drain();
        assert_eq!(batch.dropped(), 2);
        assert_eq!(batch.into_candidates().len(), UI_INPUT_QUEUE_CAPACITY);
        let empty = mailbox.drain();
        assert_eq!(empty.dropped(), 0);
        assert!(empty.into_candidates().is_empty());
    }
}
