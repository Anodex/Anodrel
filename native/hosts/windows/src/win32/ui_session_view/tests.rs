use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_notifications::NotificationMailbox;
use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{
    SessionInteractionCandidate, UiDocumentMailbox, UiDocumentSession, UiInputMailbox,
};
use anodrel_windows_file_access::WindowsFileTextService;

use super::{
    UiSessionPoll, UiSessionView, WindowFocusMailbox, WindowFullscreenMailbox,
    WindowFullscreenMode, WindowSize, WindowSizeMailbox, WindowState, WindowStateChangesMailbox,
    WindowStateMailbox, WindowStateReadMailbox, WindowTitleMailbox,
};

mod bridges;
mod isolation;

const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.root","kind":"text","value":"Connected","fontSize":16,"tone":"primary"}}"#;
const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.action","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
const SCROLL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v2","root":{"id":"session.viewport","kind":"scroll","child":{"id":"session.content","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"session.one","kind":"action","label":"One","fontSize":16,"enabled":true,"tone":"accent"},{"id":"session.two","kind":"action","label":"Two","fontSize":16,"enabled":true,"tone":"accent"},{"id":"session.three","kind":"action","label":"Three","fontSize":16,"enabled":true,"tone":"accent"}]}}}"#;
const STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"session.status","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}"#;
const UPDATED_STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"session.status","kind":"status","value":"Save failed","fontSize":16,"tone":"accent","politeness":"assertive"}}"#;

fn poll(document_changed: bool, close_requested: bool) -> UiSessionPoll {
    UiSessionPoll {
        document_changed,
        close_requested,
        changed_status: None,
    }
}

#[test]
fn applies_only_a_newer_snapshot_from_its_own_mailbox() {
    let mailbox = UiDocumentMailbox::new();
    let mut view = UiSessionView::new(
        mailbox.clone(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    let mut session = UiDocumentSession::new();
    session
        .replace_document(DOCUMENT)
        .expect("document is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));

    assert_eq!(view.poll(), poll(true, false));
    assert_eq!(view.poll(), poll(false, false));
}

#[test]
fn live_status_is_silent_initially_and_reports_only_a_later_change() {
    let mailbox = UiDocumentMailbox::new();
    let mut view = UiSessionView::new(
        mailbox.clone(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    let mut session = UiDocumentSession::new();

    session
        .replace_document_v3(STATUS_DOCUMENT)
        .expect("initial status is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false), "initial content is silent");

    session
        .replace_document_v3(STATUS_DOCUMENT)
        .expect("same status is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false), "same status is silent");

    session
        .replace_document_v3(UPDATED_STATUS_DOCUMENT)
        .expect("updated status is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(
        view.poll(),
        UiSessionPoll {
            document_changed: true,
            close_requested: false,
            changed_status: Some(ElementId::new("session.status").expect("fixed ID is valid")),
        }
    );

    session
        .replace_document(DOCUMENT)
        .expect("status removal document is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false), "removal is silent");
}

#[test]
fn accessibility_has_no_action_route_before_a_document_and_one_after() {
    let documents = UiDocumentMailbox::new();
    let mut view = UiSessionView::new(
        documents.clone(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.accessibility_action_sink().is_none());

    let mut session = UiDocumentSession::new();
    session
        .replace_document(ACTION_DOCUMENT)
        .expect("document is valid");
    documents.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false));
    assert!(view.accessibility_action_sink().is_some());
}

/// A document holding one enabled field, for the read path.
const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.field","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true}}"#;

/// A session view with a title bridge and the given validated name.
fn view_with_title(display_name: &str) -> (UiSessionView, WindowTitleMailbox) {
    let mailbox = WindowTitleMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_title(mailbox.clone(), display_name);
    (view, mailbox)
}

/// A session view with its own presentation-state bridge.
fn view_with_state() -> (UiSessionView, WindowStateMailbox) {
    let mailbox = WindowStateMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_state(mailbox.clone());
    (view, mailbox)
}

/// A session view with its own pull-only state-observation bridge.
fn view_with_state_read() -> (UiSessionView, WindowStateReadMailbox) {
    let mailbox = WindowStateReadMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_state_read(mailbox.clone());
    (view, mailbox)
}

/// A session view with its own coalesced pull-only state-change mailbox.
fn view_with_state_changes() -> (UiSessionView, WindowStateChangesMailbox) {
    let mailbox = WindowStateChangesMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_state_changes(mailbox.clone());
    (view, mailbox)
}

/// A session view with its own guarded foreground-request bridge.
fn view_with_focus() -> (UiSessionView, WindowFocusMailbox) {
    let mailbox = WindowFocusMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_focus(mailbox.clone());
    (view, mailbox)
}

/// A session view with its own reversible-fullscreen bridge.
fn view_with_fullscreen() -> (UiSessionView, WindowFullscreenMailbox) {
    let mailbox = WindowFullscreenMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_fullscreen(mailbox.clone());
    (view, mailbox)
}

/// A session view with its own bounded logical client-size bridge.
fn view_with_size() -> (UiSessionView, WindowSizeMailbox) {
    let mailbox = WindowSizeMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_window_size(mailbox.clone());
    (view, mailbox)
}

/// Proposes a title from a worker and returns what the UI thread would apply.
fn caption_for(view: &UiSessionView, mailbox: &WindowTitleMailbox, proposal: &str) -> String {
    let proposal =
        anodrel_window::WindowTitleProposal::new(proposal).expect("the proposal is valid");
    let worker = mailbox.clone();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowTitleService::set_title(&worker, &proposal)
    });
    let (request_id, caption) = loop {
        if let Some(taken) = view.take_window_title_request() {
            break taken;
        }
        std::thread::yield_now();
    };
    assert!(view.complete_window_title_request(request_id, true));
    waiting
        .join()
        .expect("the worker did not panic")
        .expect("the proposal was accepted");
    caption
}
