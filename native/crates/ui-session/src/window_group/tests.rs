//! Focused verification for portable session-window group coordination.

use std::{sync::mpsc, thread, time::Duration};

use anodrel_ui::{ElementId, UiEvent};

use super::{UiWindowGroup, UiWindowGroupError};
use crate::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox, UiWindowId};

const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Secondary","fontSize":16,"tone":"primary"}}"#;

fn take_pending(group: &UiWindowGroup<&'static str>) -> super::UiWindowOpenRequest<&'static str> {
    loop {
        if let Some(request) = group.take_open_request() {
            return request;
        }
        thread::yield_now();
    }
}

#[test]
fn commits_only_after_the_ui_thread_created_the_view() {
    let group = UiWindowGroup::new();
    let worker = group.clone();
    let (sent, received) = mpsc::channel();
    let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));

    let request = take_pending(&group);
    assert_eq!(request.context(), &"caption");
    assert_eq!(request.resources().id().to_protocol_string(), "window-1");
    assert_eq!(request.snapshot().snapshot().revision().value(), 1);
    assert!(request.resources().document_mailbox().take().is_none());
    assert!(group.complete_open(request.id(), true));

    let id = received
        .recv()
        .expect("worker returns a response")
        .expect("view opens");
    waiting
        .join()
        .expect("worker does not panic")
        .expect("worker submits its response");
    assert_eq!(id.to_protocol_string(), "window-1");
    assert!(group.contains(&id));
    assert_eq!(
        request
            .resources()
            .document_mailbox()
            .take()
            .expect("commit publishes the initial document")
            .revision()
            .value(),
        1
    );
}

#[test]
fn failed_native_creation_aborts_the_unissued_identity() {
    let group = UiWindowGroup::new();
    let worker = group.clone();
    let (sent, received) = mpsc::channel();
    let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));
    let request = take_pending(&group);
    assert!(group.fail_open(request.id()));
    assert_eq!(
        received.recv().expect("worker returns a response"),
        Err(UiWindowGroupError::Unavailable)
    );
    waiting
        .join()
        .expect("worker does not panic")
        .expect("worker submits its response");
    assert!(!group.contains(request.resources().id()));

    let worker = group.clone();
    let (sent, received) = mpsc::channel();
    let waiting = thread::spawn(move || sent.send(worker.open_secondary("retry", DOCUMENT)));
    let retry = take_pending(&group);
    assert_eq!(retry.resources().id().to_protocol_string(), "window-1");
    assert!(group.complete_open(retry.id(), true));
    assert_eq!(
        received
            .recv()
            .expect("retry returns a response")
            .expect("retry opens")
            .to_protocol_string(),
        "window-1"
    );
    waiting
        .join()
        .expect("retry worker does not panic")
        .expect("retry worker submits its response");
}

#[test]
fn keeps_one_native_creation_in_flight_and_rejects_late_completion() {
    let group = UiWindowGroup::new();
    let worker = group.clone();
    let waiting = thread::spawn(move || {
        worker.open_secondary_within("caption", DOCUMENT, Duration::from_millis(20))
    });
    let request = take_pending(&group);
    assert_eq!(
        group.open_secondary("second", DOCUMENT),
        Err(UiWindowGroupError::Busy)
    );
    assert_eq!(
        waiting.join().expect("worker does not panic"),
        Err(UiWindowGroupError::Unavailable)
    );
    assert!(
        !group.complete_open(request.id(), true),
        "a host must destroy a window created after its worker timed out"
    );
    assert!(!group.contains(&UiWindowId::parse("window-1").expect("ID parses")));
}

#[test]
fn rejects_invalid_documents_before_a_ui_thread_request_exists() {
    let group = UiWindowGroup::<&'static str>::new();
    assert!(matches!(
        group.open_secondary("caption", "not a document"),
        Err(UiWindowGroupError::DocumentRejected(_))
    ));
    assert!(group.take_open_request().is_none());
}

#[test]
fn group_shutdown_cancels_a_taken_open_without_waiting_for_its_deadline() {
    let group = UiWindowGroup::new();
    let worker = group.clone();
    let (sent, received) = mpsc::channel();
    let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));

    // Wait until the worker has reserved its view, but deliberately do not
    // hand it to a UI thread. Shutdown must answer now rather than making
    // the worker wait for the ordinary five-second host-response bound.
    loop {
        if group.take_open_request().is_some() {
            break;
        }
        thread::yield_now();
    }
    // The request was taken above, so cancellation takes the same active
    // handoff and publishes an unavailable outcome. A native thread that
    // had actually begun creating a window will instead see
    // `complete_open` return false after the cancellation.
    assert!(group.cancel_open_request());
    assert_eq!(
        received.recv().expect("worker receives cancellation"),
        Err(UiWindowGroupError::Unavailable)
    );
    waiting
        .join()
        .expect("worker does not panic")
        .expect("worker submits its response");
    assert!(!group.contains(&UiWindowId::parse("window-1").expect("fixed ID parses")));
}

#[test]
fn binds_the_existing_primary_mailboxes_into_the_group_without_copying_state() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let group = UiWindowGroup::<()>::with_primary_resources(
        document_mailbox.clone(),
        input_mailbox.clone(),
    );
    let primary = UiWindowId::primary();

    let snapshot = group
        .replace_document(&primary, DOCUMENT)
        .expect("the primary document validates");
    assert_eq!(
        document_mailbox
            .take()
            .expect("the caller-owned mailbox receives the primary snapshot")
            .revision(),
        snapshot.snapshot().revision()
    );

    input_mailbox.push(UiInputCandidate::new(
        snapshot.snapshot().revision(),
        UiEvent::ActionInvoked(ElementId::new("root").expect("fixed action ID is valid")),
    ));
    let batches = group.drain_input_batches();
    assert_eq!(batches.len(), 1);
    let (id, batch) = batches
        .into_iter()
        .next()
        .expect("the primary batch is present")
        .into_parts();
    assert_eq!(id, primary);
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.into_candidates().len(), 1);
}

#[test]
fn queues_each_secondary_close_once_and_never_queues_the_primary() {
    let group = UiWindowGroup::new();
    let worker = group.clone();
    let (sent, received) = mpsc::channel();
    let waiting = thread::spawn(move || sent.send(worker.open_secondary("caption", DOCUMENT)));
    let request = take_pending(&group);
    assert!(group.complete_open(request.id(), true));
    let secondary = received
        .recv()
        .expect("worker returns a response")
        .expect("secondary view opens");
    waiting
        .join()
        .expect("worker does not panic")
        .expect("worker submits its response");

    assert_eq!(
        group.request_secondary_close(&UiWindowId::primary()),
        Err(UiWindowGroupError::Unavailable)
    );
    assert!(group.request_secondary_close(&secondary).is_ok());
    assert!(group.request_secondary_close(&secondary).is_ok());
    assert_eq!(
        group.take_secondary_close_requests(),
        vec![secondary.clone()]
    );
    assert!(group.take_secondary_close_requests().is_empty());

    assert!(group.request_secondary_close(&secondary).is_ok());
    assert!(group.close_secondary(&secondary).is_ok());
    assert!(group.take_secondary_close_requests().is_empty());
    assert_eq!(
        group.request_secondary_close(&secondary),
        Err(UiWindowGroupError::Unavailable)
    );
}
