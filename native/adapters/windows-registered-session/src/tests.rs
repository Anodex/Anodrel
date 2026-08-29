//! Isolation checks for registered Windows session resources.

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_windows_policy::PolicyStoreError;

use super::{
    RegisteredSessionError, RegisteredSessionUi, create_registered_session, create_session,
};

#[test]
fn rejects_an_invalid_application_id_before_creating_a_pipe() {
    assert!(matches!(
        create_registered_session("org.anodrel/escape", "windows-host", "test-session"),
        Err(RegisteredSessionError::Policy(
            PolicyStoreError::InvalidApplicationId
        ))
    ));
}

#[test]
fn creates_an_owner_restricted_session_from_a_host_policy() {
    let policy = HostPolicy::new(
        "org.anodrel.sample",
        vec![Capability::DiagnosticsRead],
        "windows-host",
    )
    .expect("fixture host policy is valid");

    let (_server, invitation) = create_session(policy, "test-session").expect("session is created");
    assert!(invitation.pipe_name().starts_with(r"\\.\pipe\anodrel.v1."));
    assert_eq!(invitation.session_id(), "test-session");
}

#[test]
fn interactive_ui_resources_keep_their_close_signal_session_local() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");

    first.close_signal().request();
    assert!(first.close_signal().take());
    assert!(!second.close_signal().take());
}

#[test]
fn each_registered_session_owns_one_isolated_primary_view_group() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");
    let document = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"First","fontSize":16,"tone":"primary"}}"#;
    let primary = anodrel_ui_session::UiWindowId::primary();

    let snapshot = first
        .window_group()
        .replace_document(&primary, document)
        .expect("the primary document validates");
    assert_eq!(snapshot.snapshot().revision().value(), 1);
    assert_eq!(
        first
            .document_mailbox()
            .take()
            .expect("the first group publishes to its primary mailbox")
            .revision()
            .value(),
        1
    );
    assert!(
        second.document_mailbox().take().is_none(),
        "registered sessions must never share a primary document mailbox"
    );
}

#[test]
fn each_session_carries_its_own_title_bridge_and_validated_name() {
    // Two sessions must never share either half. A shared mailbox would let
    // one application's proposal be applied to another's window, and a
    // shared name would let a caption claim the wrong application.
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");

    assert_eq!(first.display_name(), "First Application");
    assert_eq!(second.display_name(), "Second Application");

    let proposal =
        anodrel_window::WindowTitleProposal::new("Report").expect("the proposal is valid");
    let bridge = first.window_title_mailbox();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowTitleService::set_title(&bridge, &proposal)
    });
    while first.window_title_mailbox().take().is_none() {
        std::thread::yield_now();
    }
    // The pending proposal belongs to the session that made it, and the
    // other session's bridge knows nothing about it.
    assert!(second.window_title_mailbox().take().is_none());
    assert!(first.window_title_mailbox().fail(1));
    assert!(waiting.join().expect("the worker did not panic").is_err());
}

#[test]
fn each_session_carries_its_own_closed_window_state_bridge() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");
    let bridge = first.window_state_mailbox();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowStateService::set_state(
            &bridge,
            anodrel_window::WindowState::Minimized,
        )
    });

    while first.window_state_mailbox().take().is_none() {
        assert!(
            second.window_state_mailbox().take().is_none(),
            "a session took another session's presentation command"
        );
        std::thread::yield_now();
    }
    assert!(second.window_state_mailbox().take().is_none());
    assert!(first.window_state_mailbox().fail(1));
    assert!(waiting.join().expect("the worker did not panic").is_err());
}

#[test]
fn each_session_carries_its_own_pull_only_window_state_read_bridge() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");
    let bridge = first.window_state_read_mailbox();
    let waiting =
        std::thread::spawn(move || anodrel_window::WindowStateReadService::read_state(&bridge));

    let request_id = loop {
        assert!(
            second.window_state_read_mailbox().take().is_none(),
            "a session took another session's presentation-state observation"
        );
        if let Some(request) = first.window_state_read_mailbox().take() {
            break request.id();
        }
        std::thread::yield_now();
    };
    assert!(
        first
            .window_state_read_mailbox()
            .complete(request_id, anodrel_window::WindowState::Maximized)
    );
    assert_eq!(
        waiting.join().expect("the worker did not panic"),
        Ok(anodrel_window::WindowState::Maximized)
    );
}

#[test]
fn each_session_carries_its_own_bounded_window_size_bridge() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");
    let bridge = first.window_size_mailbox();
    let waiting = std::thread::spawn(move || {
        let size = anodrel_window::WindowSize::new(800, 600).expect("fixture size is valid");
        anodrel_window::WindowSizeService::set_size(&bridge, size)
    });

    while first.window_size_mailbox().take().is_none() {
        assert!(
            second.window_size_mailbox().take().is_none(),
            "a session took another session's client-size request"
        );
        std::thread::yield_now();
    }
    assert!(second.window_size_mailbox().take().is_none());
    assert!(first.window_size_mailbox().fail(1));
    assert!(waiting.join().expect("the worker did not panic").is_err());
}

#[test]
fn each_session_carries_its_own_window_focus_bridge() {
    let first = RegisteredSessionUi::new("First Application");
    let second = RegisteredSessionUi::new("Second Application");
    let bridge = first.window_focus_mailbox();
    let waiting =
        std::thread::spawn(move || anodrel_window::WindowFocusService::request_focus(&bridge));

    while first.window_focus_mailbox().take().is_none() {
        assert!(
            second.window_focus_mailbox().take().is_none(),
            "a session took another session's focus request"
        );
        std::thread::yield_now();
    }
    assert!(second.window_focus_mailbox().take().is_none());
    assert!(first.window_focus_mailbox().fail(1));
    assert!(waiting.join().expect("the worker did not panic").is_err());
}
