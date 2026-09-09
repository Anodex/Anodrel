//! Concurrency and stale-route tests for the UI Automation focus bridge.

use std::{thread, time::Duration};

use anodrel_ui::ElementId;
use anodrel_ui_session::{UiDocumentRevision, UiDocumentSession};

use super::{UiAutomationFocusMailbox, UiAutomationFocusRequest, owner_thread};

fn id() -> ElementId {
    ElementId::new("submit").expect("fixed ID is valid")
}

fn revision() -> UiDocumentRevision {
    UiDocumentSession::new()
        .replace_document(
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Focus","fontSize":16,"tone":"primary"}}"#,
        )
        .expect("the fixed document is valid")
}

fn take_pending(mailbox: &UiAutomationFocusMailbox) -> UiAutomationFocusRequest {
    loop {
        if let Some(request) = mailbox.take() {
            return request;
        }
        thread::yield_now();
    }
}

#[test]
fn transfers_one_revision_bound_target_to_its_owner() {
    let mailbox = UiAutomationFocusMailbox::new();
    let worker = mailbox.clone();
    let waiting = thread::spawn(move || {
        worker.request_within(Some(revision()), id(), Duration::from_secs(1), || true)
    });

    let request = take_pending(&mailbox);
    assert_eq!(request.revision(), Some(revision()));
    assert_eq!(request.target(), &id());
    assert!(mailbox.complete(request.id(), true));
    assert!(waiting.join().expect("caller did not panic"));
}

#[test]
fn busy_or_unknown_routes_cannot_change_an_active_request() {
    let mailbox = UiAutomationFocusMailbox::new();
    let worker = mailbox.clone();
    let waiting = thread::spawn(move || {
        worker.request_within(Some(revision()), id(), Duration::from_secs(1), || true)
    });
    let request = take_pending(&mailbox);

    assert!(
        !mailbox.request_within(Some(revision()), id(), Duration::ZERO, || true),
        "a second automation caller entered an occupied route"
    );
    assert!(!mailbox.complete(request.id().saturating_add(1), true));
    assert!(mailbox.complete(request.id(), false));
    assert!(!waiting.join().expect("caller did not panic"));
}

#[test]
fn timeout_or_failed_wakeup_releases_the_exact_route_slot() {
    let mailbox = UiAutomationFocusMailbox::new();
    let worker = mailbox.clone();
    let waiting = thread::spawn(move || {
        worker.request_within(Some(revision()), id(), Duration::from_millis(20), || true)
    });
    let abandoned = take_pending(&mailbox);
    assert!(!waiting.join().expect("caller did not panic"));
    assert!(
        !mailbox.complete(abandoned.id(), true),
        "a late completion answered a timed-out caller"
    );
    let mut applied = false;
    assert_eq!(
        mailbox.complete_with(abandoned.id(), || {
            applied = true;
            true
        }),
        None,
        "an expired request reached host focus state"
    );
    assert!(!applied, "an expired route ran its focus decision");

    let worker = mailbox.clone();
    let next =
        thread::spawn(move || worker.request_within(None, id(), Duration::from_secs(1), || false));
    assert!(!next.join().expect("caller did not panic"));

    let worker = mailbox.clone();
    let next =
        thread::spawn(move || worker.request_within(None, id(), Duration::from_secs(1), || true));
    let request = take_pending(&mailbox);
    assert!(mailbox.complete(request.id(), true));
    assert!(next.join().expect("caller did not panic"));
}

#[test]
fn a_test_notifier_can_complete_a_route_without_a_window() {
    let mailbox = UiAutomationFocusMailbox::new();
    let route = mailbox.route(Some(revision()));
    let completing = mailbox.clone();
    let sink = route.with_notifier(move || {
        let request = completing.take().expect("route request is pending");
        completing.complete(request.id(), true)
    });

    assert!(sink.focus(id()));
}

#[test]
fn an_invalid_window_is_never_a_focus_route_owner() {
    assert_eq!(owner_thread(0), None);
}
