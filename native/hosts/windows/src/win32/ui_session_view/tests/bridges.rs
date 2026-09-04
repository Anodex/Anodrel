//! Session-local bridge and interaction checks.

use super::*;

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
fn a_session_without_a_context_menu_bridge_has_no_context_menu_route() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_context_menu_request().is_none());
    assert!(!view.complete_context_menu_request(1, true));
    assert!(view.context_menu().is_none());
}

#[test]
fn a_session_without_a_tray_bridge_has_no_tray_route() {
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    );
    assert!(view.take_tray_request().is_none());
    assert!(!view.complete_tray_request(1, true));
    assert!(view.tray().is_none());
}

#[test]
fn a_tray_model_crosses_only_its_supplied_ui_thread_mailbox() {
    let mailbox = anodrel_menu::TrayMailbox::new();
    let view = UiSessionView::new(
        UiDocumentMailbox::new(),
        UiInputMailbox::new(),
        SessionCloseSignal::default(),
        FileDialogMailbox::new(),
        WindowsFileTextService::new(),
        NotificationMailbox::new(),
    )
    .with_tray(mailbox.clone());
    let worker = mailbox.clone();
    let waiting = std::thread::spawn(move || {
        anodrel_menu::TrayService::replace(
            &worker,
            anodrel_menu::TrayRevision::INITIAL
                .next()
                .expect("the first revision exists"),
            tray_model(),
        )
    });
    let request = loop {
        if let Some(request) = view.take_tray_request() {
            break request;
        }
        std::thread::yield_now();
    };
    assert_eq!(request.revision().value(), 1);
    assert_eq!(request.model().items()[0].id().as_str(), "window.open");
    assert!(view.complete_tray_request(request.id(), true));
    assert!(waiting.join().expect("tray worker does not panic").is_ok());
}

fn tray_model() -> anodrel_menu::ContextMenuModel {
    anodrel_menu::ContextMenuModel::new(vec![anodrel_menu::MenuAction::new(
        anodrel_menu::MenuActionId::new("window.open").expect("fixed ID is valid"),
        anodrel_menu::MenuText::new("Open window").expect("fixed label is valid"),
        true,
    )])
    .expect("fixed model is valid")
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
