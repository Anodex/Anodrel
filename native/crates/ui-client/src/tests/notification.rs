//! Typed notification-operation verification.

use anodrel_json::JsonValue;

use super::{
    UiClientError, messages, request_field, request_protocol_minor, response,
    session_with_responses,
};

#[test]
fn notification_uses_the_exact_protocol_1_13_one_way_payload() {
    let (mut session, written) =
        session_with_responses([response("anodrel-ui-1", r#"{"status":"shown"}"#)]);

    session
        .show_notification("Build finished", "Two targets\nzero warnings")
        .expect("host accepted the notification");

    let messages = messages(&written);
    assert_eq!(
        request_field(&messages[1], "operation"),
        Some("notification.show".to_owned())
    );
    assert_eq!(request_protocol_minor(&messages[1]), Some(13));
    let request = JsonValue::parse(&messages[1]).expect("notification request is JSON");
    assert_eq!(
        request.as_object().and_then(|fields| fields.get("payload")),
        Some(
            &JsonValue::parse(r#"{"title":"Build finished","body":"Two targets\nzero warnings"}"#)
                .expect("expected notification payload is JSON")
        )
    );
}

#[test]
fn invalid_notification_text_stops_before_a_public_request() {
    for (title, body) in [("", "Valid body"), ("Valid title", "Bad\rbody")] {
        let (mut session, written) = session_with_responses([]);
        assert_eq!(
            session.show_notification(title, body),
            Err(UiClientError::NotificationInvalid)
        );
        assert_eq!(
            messages(&written).len(),
            1,
            "only authentication was written"
        );
    }
}

#[test]
fn a_malformed_notification_success_result_fails_closed() {
    let (mut session, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"status":"accepted"}"#)]);
    assert_eq!(
        session.show_notification("Build finished", "Two targets"),
        Err(UiClientError::ResponseInvalid)
    );
}
