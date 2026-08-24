//! Contract coverage for the typed targetless session-window controls.

use crate::{WindowFullscreenMode, WindowSize, WindowState};

use super::{
    JsonValue, UiClientError, messages, request_field, request_protocol_minor, response,
    session_with_responses,
};

#[test]
fn window_controls_use_their_minimum_versions_and_exact_payloads() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"status":"applied"}"#),
        response("anodrel-ui-2", r#"{"status":"applied"}"#),
        response("anodrel-ui-3", r#"{"status":"requested"}"#),
        response("anodrel-ui-4", r#"{"status":"applied"}"#),
        response("anodrel-ui-5", r#"{"status":"applied"}"#),
    ]);

    session
        .set_window_title("Quarterly report")
        .expect("title proposal is accepted");
    session
        .set_window_state(WindowState::Maximized)
        .expect("closed state is accepted");
    session
        .request_window_focus()
        .expect("foreground request is accepted");
    session
        .set_window_fullscreen(WindowFullscreenMode::Fullscreen)
        .expect("fullscreen request is accepted");
    session
        .set_window_size(WindowSize::new(800, 600).expect("fixture size is valid"))
        .expect("size request is accepted");

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("window.title.set".to_owned()),
            Some("window.state.set".to_owned()),
            Some("window.focus.request".to_owned()),
            Some("window.fullscreen.set".to_owned()),
            Some("window.size.set".to_owned()),
        ]
    );
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_protocol_minor(message))
            .collect::<Vec<_>>(),
        [Some(14), Some(16), Some(20), Some(21), Some(23)]
    );

    let payloads = messages
        .iter()
        .skip(1)
        .map(|message| {
            JsonValue::parse(message)
                .expect("request is JSON")
                .as_object()
                .and_then(|fields| fields.get("payload"))
                .cloned()
                .expect("request has a payload")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        payloads,
        [
            JsonValue::parse(r#"{"title":"Quarterly report"}"#).expect("fixture is JSON"),
            JsonValue::parse(r#"{"state":"maximized"}"#).expect("fixture is JSON"),
            JsonValue::Object(Default::default()),
            JsonValue::parse(r#"{"mode":"fullscreen"}"#).expect("fixture is JSON"),
            JsonValue::parse(r#"{"width":800,"height":600}"#).expect("fixture is JSON"),
        ]
    );
}

#[test]
fn window_controls_reject_an_unexpected_acceptance_shape() {
    let (mut session, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"status":"requested"}"#)]);

    assert_eq!(
        session.set_window_title("Quarterly report"),
        Err(UiClientError::ResponseInvalid),
        "a title proposal must not accept the focus result shape"
    );
}

#[test]
fn invalid_window_title_stops_before_a_request_is_created() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.set_window_title("unsafe\nwindow title"),
        Err(UiClientError::WindowTitleInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}
