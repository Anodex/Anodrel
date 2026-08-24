use super::support::*;
use crate::*;

#[test]
fn notifications_need_their_own_grant_and_protocol_minor() {
    let payload = notification_payload("Build finished", "Two targets");

    // No grant.
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new("test.application", vec![], "test-host").expect("policy is valid"),
            HostServices::unavailable().with_notifications(RecordingNotifications::default()),
        )
        .handle_json(&request_v1_13("notification.show", &payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    // Granted, but asked for at a protocol minor that predates the
    // operation: an older client must not reach a newer capability.
    let unsupported = JsonValue::parse(
        &host_with_notifications(RecordingNotifications::default())
            .handle_json(&request_v1_12("notification.show", &payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn a_granted_notification_reaches_the_service_unchanged() {
    let service = RecordingNotifications::default();
    let response = JsonValue::parse(
        &host_with_notifications(service).handle_json(&request_v1_13(
            "notification.show",
            &notification_payload("Done", "All green"),
        )),
    )
    .expect("response JSON is valid");

    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&response, "result"), "status").as_string(),
        Some("shown")
    );
}

#[test]
fn a_rejected_notification_never_echoes_its_own_text_back() {
    // A refusal must not become a way to have the host repeat content.
    let response = JsonValue::parse(
        &host_with_notifications(RecordingNotifications::default()).handle_json(&request_v1_13(
            "notification.show",
            &notification_payload("Spoofed\rsecond line", "body"),
        )),
    )
    .expect("response JSON is valid");

    let error = field(&response, "error");
    assert_eq!(
        field(error, "code").as_string(),
        Some("notification.text_invalid")
    );
    assert!(!response.to_json().contains("Spoofed"));
}

#[test]
fn notification_payloads_accept_exactly_a_title_and_a_body() {
    let host = host_with_notifications(RecordingNotifications::default());
    for payload in [
        r#"{"title":"only"}"#,
        r#"{"body":"only"}"#,
        // An extra field is a mismatch, not something to ignore, so a
        // future urgency or action field cannot be smuggled past 1.13.
        r#"{"title":"a","body":"b","urgency":"high"}"#,
        r#"{"title":"a","body":2}"#,
    ] {
        let response =
            JsonValue::parse(&host.handle_json(&request_v1_13("notification.show", payload)))
                .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }
}

#[test]
fn service_failures_stay_distinguishable_without_describing_the_user() {
    for (error, expected) in [
        (
            NotificationServiceError::Unavailable,
            "notification.unavailable",
        ),
        (NotificationServiceError::Busy, "notification.busy"),
    ] {
        let response = JsonValue::parse(
            &host_with_notifications(RecordingNotifications::failing(error)).handle_json(
                &request_v1_13(
                    "notification.show",
                    &notification_payload("Done", "All green"),
                ),
            ),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some(expected)
        );
    }
}
