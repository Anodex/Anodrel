//! Baseline typed-session protocol coverage.

use super::{
    DOCUMENT, JsonValue, event_batch, messages, request_field, response, session_with_responses,
};

#[test]
fn the_typed_session_uses_only_its_three_documented_operations() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", &event_batch("template.complete", "1")),
        response("anodrel-ui-3", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_document_v1(DOCUMENT)
            .expect("document is accepted")
            .value(),
        1
    );
    let batch = session.read_actions().expect("action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    assert_eq!(batch.actions().len(), 1);
    assert_eq!(batch.actions()[0].action(), "template.complete");
    assert_eq!(batch.actions()[0].revision().value(), 1);
    session.close().expect("close is accepted");

    let messages = messages(&written);
    let operations = messages
        .iter()
        .skip(1)
        .map(|message| request_field(message, "operation"))
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            Some("ui.document.replace".to_owned()),
            Some("ui.events.read".to_owned()),
            Some("session.close".to_owned()),
        ]
    );
    let request_ids = messages
        .iter()
        .skip(1)
        .map(|message| request_field(message, "requestId"))
        .collect::<Vec<_>>();
    assert_eq!(
        request_ids,
        [
            Some("anodrel-ui-1".to_owned()),
            Some("anodrel-ui-2".to_owned()),
            Some("anodrel-ui-3".to_owned()),
        ]
    );
    assert!(messages.iter().skip(1).all(|message| {
        JsonValue::parse(message)
            .expect("request is JSON")
            .as_object()
            .and_then(|fields| fields.get("protocolVersion"))
            .and_then(JsonValue::as_object)
            .and_then(|version| version.get("minor"))
            .and_then(JsonValue::as_u16)
            == Some(3)
    }));
}
