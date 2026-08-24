use super::super::support::*;
use crate::*;

#[test]
fn accepts_only_a_granted_current_protocol_session_close_request() {
    let signal = SessionCloseSignal::default();
    let close_host = CoreHost::with_session_components(
        HostPolicy::new(
            "test.application",
            vec![Capability::SessionClose],
            "test-host",
        )
        .expect("test policy is valid"),
        UiInputMailbox::new(),
        signal.clone(),
    );
    let accepted = JsonValue::parse(&close_host.handle_json(&request_v1_3("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&accepted, "result"), "status").as_string(),
        Some("accepted")
    );
    assert!(signal.take());
    assert!(!signal.take());

    let denied = JsonValue::parse(&host(vec![]).handle_json(&request_v1_3("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let old_minor = JsonValue::parse(&close_host.handle_json(&request_v1_2("session.close", "{}")))
        .expect("response is JSON");
    assert_eq!(
        field(field(&old_minor, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
