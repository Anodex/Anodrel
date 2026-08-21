//! A compiled diagnostic for one interactive native Anodrel session.
//!
//! The child owns one fixed document and waits for its one fixed semantic
//! action. It proves authenticated document delivery, action pull, and
//! session-close behaviour without Node.js, a webview, or machine trust.
//! It accepts no arguments, configuration, files, network input, or content.

#![deny(unsafe_op_in_unsafe_fn)]

mod document;
mod stages;

use std::{io, process::ExitCode, thread};

use anodrel_client::{Client, InteractivePollSchedule, ProtocolVersion};
use anodrel_json::JsonValue;
use anodrel_windows_client::WindowsClientStream;

use document::{NATIVE_UI_ACTION, NATIVE_UI_DOCUMENT};
use stages::Stage;

/// `ui.document.replace`, `ui.events.read`, and `session.close` require these
/// first three compatible protocol minors respectively.
const NATIVE_UI_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(3);

type NativeUiClient = Client<WindowsClientStream>;

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let Ok(invitation) = NativeUiClient::read_invitation(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Ok(stream) = WindowsClientStream::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    let Ok(mut session) = NativeUiClient::authenticate(stream, invitation) else {
        return Stage::AuthenticationRejected;
    };

    if !delivers_first_document(&mut session) {
        return Stage::DocumentRejected;
    }
    match wait_for_native_ui_action(&mut session) {
        Stage::Completed => {}
        stage => return stage,
    }
    if !closes_session(&mut session) {
        return Stage::CloseRejected;
    }
    Stage::Completed
}

fn delivers_first_document(session: &mut NativeUiClient) -> bool {
    let payload = JsonValue::Object(
        [(
            "document".to_owned(),
            JsonValue::String(NATIVE_UI_DOCUMENT.to_owned()),
        )]
        .into_iter()
        .collect(),
    );
    request(
        session,
        "native-ui-document",
        "ui.document.replace",
        payload,
    )
    .and_then(|result| {
        field(&result, "revision")
            .and_then(JsonValue::as_string)
            .map(str::to_owned)
    })
    .as_deref()
        == Some("1")
}

/// Polls the host-validated event surface until the one rendered action is
/// returned. A foreign action, overflow, discard, or malformed response ends
/// the diagnostic rather than letting it treat another UI as its own.
fn wait_for_native_ui_action(session: &mut NativeUiClient) -> Stage {
    for interval in InteractivePollSchedule::new() {
        let Some(result) = request(
            session,
            "native-ui-events",
            "ui.events.read",
            JsonValue::Object(Default::default()),
        ) else {
            return Stage::EventReadFailed;
        };
        if number(&result, "dropped") != Some(0) || number(&result, "discarded") != Some(0) {
            return Stage::EventReadFailed;
        }
        let Some(JsonValue::Array(events)) = field(&result, "events") else {
            return Stage::EventReadFailed;
        };
        match events.first() {
            None => thread::sleep(interval),
            Some(event) if is_native_ui_action(event) => return Stage::Completed,
            Some(_) => return Stage::EventReadFailed,
        }
    }
    Stage::ActionNotObserved
}

fn is_native_ui_action(event: &JsonValue) -> bool {
    let Some(payload) = field(event, "payload") else {
        return false;
    };
    field(event, "eventName").and_then(JsonValue::as_string) == Some("ui.action.invoked")
        && field(payload, "action").and_then(JsonValue::as_string) == Some(NATIVE_UI_ACTION)
        && field(payload, "revision").and_then(JsonValue::as_string) == Some("1")
}

fn closes_session(session: &mut NativeUiClient) -> bool {
    request(
        session,
        "native-ui-close",
        "session.close",
        JsonValue::Object(Default::default()),
    )
    .and_then(|result| {
        field(&result, "status")
            .and_then(JsonValue::as_string)
            .map(str::to_owned)
    })
    .as_deref()
        == Some("accepted")
}

fn request(
    session: &mut NativeUiClient,
    request_id: &str,
    operation: &str,
    payload: JsonValue,
) -> Option<JsonValue> {
    session
        .request(NATIVE_UI_PROTOCOL, request_id, operation, payload)
        .ok()
}

fn number(value: &JsonValue, name: &str) -> Option<u16> {
    field(value, name).and_then(JsonValue::as_u16)
}

fn field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(name)
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{NATIVE_UI_PROTOCOL, is_native_ui_action, number};

    #[test]
    fn the_requested_minor_covers_every_native_ui_operation() {
        assert_eq!(NATIVE_UI_PROTOCOL.minor(), 3);
    }

    #[test]
    fn accepts_only_its_action_at_its_first_document_revision() {
        let event = |action: &str, revision: &str| {
            JsonValue::parse(&format!(
                r#"{{"kind":"event","eventName":"ui.action.invoked","payload":{{"revision":"{revision}","action":"{action}"}}}}"#
            ))
            .expect("the diagnostic event is JSON")
        };

        assert!(is_native_ui_action(&event("native.ui.complete", "1")));
        assert!(!is_native_ui_action(&event("native.ui.other", "1")));
        assert!(!is_native_ui_action(&event("native.ui.complete", "2")));
        assert!(!is_native_ui_action(&JsonValue::Null));
    }

    #[test]
    fn reads_only_bounded_event_counts() {
        let result = JsonValue::parse(r#"{"events":[],"dropped":0,"discarded":3}"#)
            .expect("the diagnostic result is JSON");
        assert_eq!(number(&result, "dropped"), Some(0));
        assert_eq!(number(&result, "discarded"), Some(3));
        assert_eq!(number(&result, "missing"), None);
    }
}
