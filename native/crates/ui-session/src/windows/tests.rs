//! Focused verification for portable session-window resources and lifecycles.

use anodrel_ui::{ElementId, UiEvent};

use super::{MAX_SESSION_WINDOWS, UiWindowId, UiWindowSessionError, UiWindowSessions};
use crate::UiWindowIdError;

const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
const OTHER_ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"other","kind":"action","label":"Other","fontSize":16,"enabled":true,"tone":"accent"}}"#;

fn continue_event() -> UiEvent {
    UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid"))
}

#[test]
fn recognizes_only_canonical_session_window_identities() {
    assert_eq!(
        UiWindowId::parse("main").expect("primary ID parses"),
        UiWindowId::primary()
    );
    assert_eq!(
        UiWindowId::parse("window-17")
            .expect("secondary ID parses")
            .to_protocol_string(),
        "window-17"
    );
    for invalid in [
        "",
        "Main",
        "window-0",
        "window-01",
        "window--1",
        "window-65536",
    ] {
        assert_eq!(UiWindowId::parse(invalid), Err(UiWindowIdError::Invalid));
    }
}

#[test]
fn commits_a_valid_secondary_only_after_host_creation_succeeds() {
    let mut windows = UiWindowSessions::new();
    let pending = windows
        .prepare_secondary(ACTION_DOCUMENT)
        .expect("initial document validates");
    let resources = pending.resources();
    assert_eq!(pending.id().to_protocol_string(), "window-1");
    assert_eq!(pending.snapshot().revision().value(), 1);
    assert_eq!(windows.open_count(), 1);
    assert!(!windows.contains(pending.id()));
    assert!(resources.document_mailbox().take().is_none());

    let committed = windows
        .commit_secondary(pending)
        .expect("host registration commits the view");
    assert_eq!(windows.open_count(), 2);
    assert!(windows.contains(committed.id()));
    assert_eq!(committed.snapshot().revision().value(), 1);
    assert_eq!(
        resources
            .document_mailbox()
            .take()
            .expect("commit publishes the first snapshot")
            .revision()
            .value(),
        1
    );
}

#[test]
fn keeps_revisions_and_semantic_actions_independent_per_view() {
    let mut windows = UiWindowSessions::new();
    let first_pending = windows
        .prepare_secondary(ACTION_DOCUMENT)
        .expect("first document validates");
    let first = windows
        .commit_secondary(first_pending)
        .expect("first view commits")
        .id()
        .clone();
    let second_pending = windows
        .prepare_secondary(OTHER_ACTION_DOCUMENT)
        .expect("second document validates");
    let second = windows
        .commit_secondary(second_pending)
        .expect("second view commits")
        .id()
        .clone();

    let replacement = windows
        .replace_document(&first, ACTION_DOCUMENT)
        .expect("first view updates");
    assert_eq!(replacement.snapshot().revision().value(), 2);
    assert_eq!(
        windows
            .accept_event(&first, replacement.snapshot().revision(), continue_event())
            .expect("first current action is accepted")
            .revision()
            .value(),
        2
    );
    let second_replacement = windows
        .replace_document(&second, OTHER_ACTION_DOCUMENT)
        .expect("second view updates independently");
    assert_eq!(
        second_replacement.snapshot().revision(),
        replacement.snapshot().revision(),
        "equal numeric revisions remain scoped to different views"
    );
    assert_eq!(
        windows.accept_event(
            &second,
            second_replacement.snapshot().revision(),
            continue_event()
        ),
        Err(UiWindowSessionError::EventRejected(
            super::UiSessionError::ActionUnavailable
        ))
    );
}

#[test]
fn failed_open_never_consumes_an_identity_or_mutates_the_group() {
    let mut windows = UiWindowSessions::new();
    assert!(matches!(
        windows.prepare_secondary("not a document"),
        Err(UiWindowSessionError::DocumentRejected(_))
    ));
    let pending = windows
        .prepare_secondary(ACTION_DOCUMENT)
        .expect("a valid first request still reserves the first identity");
    assert_eq!(pending.id().to_protocol_string(), "window-1");
    assert_eq!(windows.open_count(), 1);
}

#[test]
fn enforces_a_small_open_set_and_never_reuses_a_closed_identity() {
    let mut windows = UiWindowSessions::new();
    let mut ids = Vec::new();
    for expected in 1..MAX_SESSION_WINDOWS {
        let pending = windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("capacity remains");
        assert_eq!(
            pending.id().to_protocol_string(),
            format!("window-{expected}")
        );
        ids.push(
            windows
                .commit_secondary(pending)
                .expect("view commits")
                .id()
                .clone(),
        );
    }
    assert_eq!(windows.open_count(), MAX_SESSION_WINDOWS);
    assert!(matches!(
        windows.prepare_secondary(ACTION_DOCUMENT),
        Err(UiWindowSessionError::OpenLimitReached)
    ));

    windows
        .close_secondary(&ids[1])
        .expect("a secondary can close");
    assert!(!windows.contains(&ids[1]));
    assert!(matches!(
        windows.replace_document(&ids[1], ACTION_DOCUMENT),
        Err(UiWindowSessionError::WindowUnavailable)
    ));
    let next = windows
        .prepare_secondary(ACTION_DOCUMENT)
        .expect("released capacity allows a different identity");
    assert_eq!(next.id().to_protocol_string(), "window-4");
    assert_eq!(
        windows.close_secondary(&UiWindowId::primary()),
        Err(UiWindowSessionError::PrimaryCannotClose)
    );
}

#[test]
fn admits_only_one_pending_native_creation_and_allows_explicit_rollback() {
    let mut windows = UiWindowSessions::new();
    let pending = windows
        .prepare_secondary(ACTION_DOCUMENT)
        .expect("first native creation reserves its identity");
    assert!(matches!(
        windows.prepare_secondary(ACTION_DOCUMENT),
        Err(UiWindowSessionError::OpenBusy)
    ));
    windows
        .abort_secondary(pending)
        .expect("failed native creation releases its reservation");
    assert_eq!(
        windows
            .prepare_secondary(ACTION_DOCUMENT)
            .expect("the same unissued identity may be retried")
            .id()
            .to_protocol_string(),
        "window-1"
    );
}
