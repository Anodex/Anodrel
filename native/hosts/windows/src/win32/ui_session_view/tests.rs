//! Focused verification for authenticated native UI-session view state.

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

#[test]
fn the_caption_a_session_applies_always_ends_with_its_validated_name() {
    // This is the impersonation guard at the point it actually matters: the
    // string handed to User32. Whatever the application proposes, the
    // caption still names the application the host validated.
    let (view, mailbox) = view_with_title("Anodrel Sample");

    assert_eq!(
        caption_for(&view, &mailbox, "Quarterly Report.pdf"),
        "Quarterly Report.pdf \u{2014} Anodrel Sample"
    );
    assert_eq!(
        caption_for(&view, &mailbox, "Windows Security"),
        "Windows Security \u{2014} Anodrel Sample"
    );
    // Even a proposal that already carries the separator cannot end the
    // caption before the real name.
    assert!(
        caption_for(&view, &mailbox, "Report \u{2014} Some Other App")
            .ends_with(" \u{2014} Anodrel Sample")
    );
}

#[test]
fn a_granted_read_returns_the_text_a_person_actually_typed() {
    // The whole path in one test: a document seeds a field, a person types
    // into the host's state, and a read crossing the UI-thread bridge
    // returns exactly that text.
    let mailbox = anodrel_ui_session::UiFieldMailbox::new();
    let documents = UiDocumentMailbox::new();
    let mut view = UiSessionView::new(
        documents.clone(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_field_reads(mailbox.clone());

    // Delivered the way a real session delivers one, through the mailbox.
    let mut session = UiDocumentSession::new();
    session
        .replace_document(FIELD_DOCUMENT)
        .expect("document is valid");
    documents.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false));

    let width = 920.0;
    let height = 660.0;
    view.focus_next(width, height);
    for character in "Ada".chars() {
        assert!(view.type_character(width, height, character));
    }

    let worker = mailbox.clone();
    let waiting = std::thread::spawn(move || anodrel_ui_session::UiFieldReader::read(&worker));
    let request_id = loop {
        if let Some(id) = view.take_field_read() {
            break id;
        }
        std::thread::yield_now();
    };
    assert!(view.complete_field_read(request_id));

    let snapshot = waiting
        .join()
        .expect("the worker did not panic")
        .expect("the read succeeded");
    assert_eq!(snapshot.fields().len(), 1);
    assert_eq!(snapshot.fields()[0].id().as_str(), "session.field");
    assert_eq!(snapshot.fields()[0].value(), "Ada");
}

#[test]
fn a_session_without_a_field_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_field_read().is_none());
    assert!(!view.complete_field_read(1));
}

#[test]
fn a_session_without_a_title_bridge_answers_nothing_and_completes_nothing() {
    // The diagnostic session view has no bridge. It must not panic, and it
    // must not claim to have completed a request it never had.
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_title_request().is_none());
    assert!(!view.complete_window_title_request(1, true));
}

#[test]
fn a_session_without_a_state_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_state_request().is_none());
    assert!(!view.complete_window_state_request(1, true));
}

#[test]
fn a_session_without_a_state_read_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_state_read_request().is_none());
    assert!(!view.complete_window_state_read_request(1, Some(WindowState::Restored)));
}

#[test]
fn a_session_without_a_state_change_mailbox_records_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(!view.record_window_state_change(WindowState::Restored));
}

#[test]
fn a_session_without_a_focus_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_focus_request().is_none());
    assert!(!view.complete_window_focus_request(1, true));
}

#[test]
fn a_session_without_a_fullscreen_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_fullscreen_request().is_none());
    assert!(!view.complete_window_fullscreen_request(1, true));
    assert!(view.fullscreen_restore().is_none());
}

#[test]
fn a_session_without_a_size_bridge_answers_nothing_and_completes_nothing() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_window_size_request().is_none());
    assert!(!view.complete_window_size_request(1, true));
}

#[test]
fn a_session_without_a_menu_bridge_has_no_menu_request_or_command_route() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_menu_request().is_none());
    assert!(!view.complete_menu_request(1, true));
    assert!(!view.offer_menu_command(0x7000, 0));
    assert!(!view.offer_menu_shortcut(b'M'.into(), true, true, false));
}

#[test]
fn consumes_only_its_supplied_session_close_signal() {
    let signal = SessionCloseSignal::default();
    let mut view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        signal.clone(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );

    assert_eq!(view.poll(), poll(false, false));
    signal.request();
    assert_eq!(view.poll(), poll(false, true));
    assert_eq!(view.poll(), poll(false, false));
}

#[test]
fn queues_a_focused_action_only_with_the_current_document_revision() {
    let mailbox = UiDocumentMailbox::new();
    let inputs = UiInputMailbox::new();
    let mut view = UiSessionView::new(
        mailbox.clone(),
        inputs.clone(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    let mut session = UiDocumentSession::new();
    session
        .replace_document(ACTION_DOCUMENT)
        .expect("document is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false));

    assert!(view.focus_next(920.0, 660.0));
    assert!(view.activate_focused(920.0, 660.0));
    let batch = inputs.drain();
    assert_eq!(batch.dropped(), 0);
    let candidates = batch.into_candidates();
    assert_eq!(candidates.len(), 1);
    let SessionInteractionCandidate::Ui(candidate) = candidates
        .into_iter()
        .next()
        .expect("one action candidate exists")
    else {
        panic!("native focus activation must produce a document candidate");
    };
    let (revision, UiEvent::ActionInvoked(action)) = candidate.into_parts();
    assert_eq!(revision.value(), 1);
    assert_eq!(action.as_str(), "session.action");
}

#[test]
fn scrolls_an_explicit_version_two_snapshot_only_in_local_view_state() {
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
        .replace_document_v2(SCROLL_DOCUMENT)
        .expect("version two document is valid");
    mailbox.publish(session.snapshot().expect("snapshot is available"));
    assert_eq!(view.poll(), poll(true, false));

    assert!(view.scroll_page(920.0, 70.0, true));
    assert_eq!(view.revision.value(), 1);
}
