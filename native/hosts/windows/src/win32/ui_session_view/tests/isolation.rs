//! Cross-session bridge isolation checks.

use super::*;

#[test]
fn one_session_cannot_take_another_sessions_state_command() {
    let (first, first_mailbox) = view_with_state();
    let (second, _second_mailbox) = view_with_state();
    let worker = first_mailbox.clone();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowStateService::set_state(&worker, WindowState::Maximized)
    });

    let (request_id, state) = loop {
        assert!(
            second.take_window_state_request().is_none(),
            "a session took another session's state command"
        );
        if let Some(request) = first.take_window_state_request() {
            break request;
        }
        std::thread::yield_now();
    };
    assert_eq!(state, WindowState::Maximized);
    assert!(first.complete_window_state_request(request_id, true));
    assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
}

#[test]
fn one_session_cannot_take_another_sessions_state_observation() {
    let (first, first_mailbox) = view_with_state_read();
    let (second, _second_mailbox) = view_with_state_read();
    let worker = first_mailbox.clone();
    let waiting =
        std::thread::spawn(move || anodrel_window::WindowStateReadService::read_state(&worker));

    let request_id = loop {
        assert!(
            second.take_window_state_read_request().is_none(),
            "a session took another session's state observation"
        );
        if let Some(request_id) = first.take_window_state_read_request() {
            break request_id;
        }
        std::thread::yield_now();
    };
    assert!(first.complete_window_state_read_request(request_id, Some(WindowState::Maximized)));
    assert_eq!(
        waiting.join().expect("the worker did not panic"),
        Ok(WindowState::Maximized)
    );
}

#[test]
fn one_session_cannot_record_another_sessions_state_change() {
    let (first, first_mailbox) = view_with_state_changes();
    let (second, second_mailbox) = view_with_state_changes();

    assert!(first.record_window_state_change(WindowState::Restored));
    assert!(second.record_window_state_change(WindowState::Restored));
    assert!(first.record_window_state_change(WindowState::Maximized));

    assert_eq!(
        anodrel_window::WindowStateChangesService::read_change(&first_mailbox),
        Ok(Some(WindowState::Maximized))
    );
    assert_eq!(
        anodrel_window::WindowStateChangesService::read_change(&second_mailbox),
        Ok(None)
    );
}

#[test]
fn one_session_cannot_take_another_sessions_focus_request() {
    let (first, first_mailbox) = view_with_focus();
    let (second, _second_mailbox) = view_with_focus();
    let worker = first_mailbox.clone();
    let waiting =
        std::thread::spawn(move || anodrel_window::WindowFocusService::request_focus(&worker));

    let request_id = loop {
        assert!(
            second.take_window_focus_request().is_none(),
            "a session took another session's focus request"
        );
        if let Some(request_id) = first.take_window_focus_request() {
            break request_id;
        }
        std::thread::yield_now();
    };
    assert!(first.complete_window_focus_request(request_id, true));
    assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
}

#[test]
fn one_session_cannot_take_another_sessions_fullscreen_request() {
    let (first, first_mailbox) = view_with_fullscreen();
    let (second, _second_mailbox) = view_with_fullscreen();
    let worker = first_mailbox.clone();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowFullscreenService::set_fullscreen(
            &worker,
            WindowFullscreenMode::Fullscreen,
        )
    });

    let (request_id, mode) = loop {
        assert!(
            second.take_window_fullscreen_request().is_none(),
            "a session took another session's fullscreen request"
        );
        if let Some(request) = first.take_window_fullscreen_request() {
            break request;
        }
        std::thread::yield_now();
    };
    assert_eq!(mode, WindowFullscreenMode::Fullscreen);
    assert!(first.complete_window_fullscreen_request(request_id, true));
    assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
}

#[test]
fn one_session_cannot_take_another_sessions_size_request() {
    let (first, first_mailbox) = view_with_size();
    let (second, _second_mailbox) = view_with_size();
    let worker = first_mailbox.clone();
    let size = WindowSize::new(800, 600).expect("fixture size is valid");
    let waiting =
        std::thread::spawn(move || anodrel_window::WindowSizeService::set_size(&worker, size));

    let (request_id, requested) = loop {
        assert!(
            second.take_window_size_request().is_none(),
            "a session took another session's client-size request"
        );
        if let Some(request) = first.take_window_size_request() {
            break request;
        }
        std::thread::yield_now();
    };
    assert_eq!(requested, size);
    assert!(first.complete_window_size_request(request_id, true));
    assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
}

#[test]
fn one_session_cannot_take_another_sessions_title_proposal() {
    // Each view holds its own bridge, so a proposal made to one session is
    // invisible to every other window in the same message loop.
    let (first, first_mailbox) = view_with_title("First Application");
    let (second, _second_mailbox) = view_with_title("Second Application");

    let proposal =
        anodrel_window::WindowTitleProposal::new("Report").expect("the proposal is valid");
    let worker = first_mailbox.clone();
    let waiting = std::thread::spawn(move || {
        anodrel_window::WindowTitleService::set_title(&worker, &proposal)
    });
    while {
        let pending = second.take_window_title_request();
        assert!(pending.is_none(), "a session took another's proposal");
        first_mailbox.clone().take().is_none()
    } {
        std::thread::yield_now();
    }
    assert!(first.complete_window_title_request(1, false));
    assert!(waiting.join().expect("the worker did not panic").is_err());
}
