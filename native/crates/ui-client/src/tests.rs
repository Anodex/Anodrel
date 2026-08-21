use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_client::Client;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use crate::{UiActionBatch, UiClientError, UiEvent, UiSession};

const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.ui-client-test";
const SESSION_ID: &str = "ui-client-test-session";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"action","label":"Complete","fontSize":16,"enabled":true,"tone":"accent"}}"#;
const MENU: &str = r#"{"menus":[{"label":"File","items":[{"id":"template.menu.complete","label":"Complete menu template session","enabled":true}]}]}"#;

type WriteLog = Arc<Mutex<Vec<Vec<u8>>>>;

#[derive(Debug)]
struct TestStream {
    reads: VecDeque<Vec<u8>>,
    written: WriteLog,
}

impl TestStream {
    fn new(reads: impl IntoIterator<Item = Vec<u8>>, written: WriteLog) -> Self {
        Self {
            reads: reads.into_iter().collect(),
            written,
        }
    }
}

impl Read for TestStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let Some(next) = self.reads.pop_front() else {
            return Ok(0);
        };
        assert!(next.len() <= output.len(), "test chunk must fit the buffer");
        output[..next.len()].copy_from_slice(&next);
        Ok(next.len())
    }
}

impl Write for TestStream {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.written
            .lock()
            .expect("test write log is available")
            .push(input.to_owned());
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

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

#[test]
fn native_menu_session_uses_the_fixed_protocol_1_18_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", r#"{"revision":"1"}"#),
        response(
            "anodrel-ui-3",
            &menu_event_batch("template.menu.complete", "1"),
        ),
        response("anodrel-ui-4", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_document_v1(DOCUMENT)
            .expect("document is accepted")
            .value(),
        1
    );
    assert_eq!(
        session
            .replace_menu_v1(MENU)
            .expect("menu is accepted")
            .value(),
        1
    );
    let batch = session.read_events().expect("menu event batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [UiEvent::MenuAction(action)] = batch.events() else {
        panic!("the fixed native menu event is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.menu.complete");
    session.close().expect("close is accepted");

    let messages = messages(&written);
    let minors = messages
        .iter()
        .skip(1)
        .map(|message| request_protocol_minor(message))
        .collect::<Vec<_>>();
    assert_eq!(minors, [Some(3), Some(18), Some(18), Some(3)]);
    let menu_request = JsonValue::parse(&messages[2]).expect("menu request is JSON");
    assert_eq!(
        menu_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(MENU).expect("fixed menu is JSON"))
    );
}

#[test]
fn document_only_event_reads_fail_closed_when_a_menu_event_arrives() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &menu_event_batch("template.menu.complete", "1"),
    )]);

    assert_eq!(session.read_actions(), Err(UiClientError::ResponseInvalid));
}

#[test]
fn unexpected_or_invalid_events_never_become_typed_actions() {
    let malformed = r#"{"events":[{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{"major":1,"minor":2},"schemaVersion":{"major":1,"minor":0},"payload":{"revision":"1","action":"not valid"}}],"dropped":0,"discarded":0}"#;
    let (mut session, _) = session_with_responses([response("anodrel-ui-1", malformed)]);

    assert_eq!(session.read_actions(), Err(UiClientError::ResponseInvalid));
}

#[test]
fn invalid_documents_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_document_v1("not a document"),
        Err(UiClientError::DocumentInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn invalid_menus_fail_before_they_can_create_a_protocol_request() {
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_menu_v1(r#"{"menus":[]}"#),
        Err(UiClientError::MenuInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn menu_revisions_must_be_nonzero_canonical_decimal_values() {
    let (mut session, _) =
        session_with_responses([response("anodrel-ui-1", r#"{"revision":"01"}"#)]);

    assert_eq!(
        session.replace_menu_v1(MENU),
        Err(UiClientError::ResponseInvalid)
    );
}

#[test]
fn documents_over_the_operation_limit_fail_before_they_can_create_a_request() {
    let too_large_but_otherwise_valid = format!(
        r#"{{"format":"anodrel.ui.document.v1","root":{{"id":"root","kind":"text","value":"{}","fontSize":16,"tone":"primary"}}}}"#,
        "x".repeat(24 * 1024)
    );
    assert!(
        anodrel_ui_document::decode(&too_large_but_otherwise_valid).is_ok(),
        "the operation limit, not the document codec, must reject this fixture"
    );
    let (mut session, written) = session_with_responses([]);

    assert_eq!(
        session.replace_document_v1(&too_large_but_otherwise_valid),
        Err(UiClientError::DocumentInvalid)
    );
    assert_eq!(
        messages(&written).len(),
        1,
        "only authentication was written"
    );
}

#[test]
fn batches_refuse_noncanonical_counts_and_revisions() {
    let invalid_count =
        JsonValue::parse(r#"{"events":[],"dropped":-1,"discarded":0}"#).expect("fixture is JSON");
    assert_eq!(
        UiActionBatch::parse(&invalid_count),
        Err(UiClientError::ResponseInvalid)
    );
    let invalid_revision =
        JsonValue::parse(&event_batch("template.complete", "01")).expect("fixture is JSON");
    assert_eq!(
        UiActionBatch::parse(&invalid_revision),
        Err(UiClientError::ResponseInvalid)
    );
}

fn session_with_responses(
    responses: impl IntoIterator<Item = String>,
) -> (UiSession<TestStream>, WriteLog) {
    let written = Arc::new(Mutex::new(Vec::new()));
    let mut frames = vec![frame(r#"{"kind":"session.authenticated"}"#)];
    frames.extend(responses.into_iter().map(|response| frame(&response)));
    let stream = TestStream::new(frames, Arc::clone(&written));
    let invitation =
        BootstrapInvitation::new(PIPE_NAME, SESSION_ID, TOKEN).expect("invitation is valid");
    let client = Client::authenticate(stream, invitation).expect("authentication succeeds");
    (UiSession::new(client), written)
}

fn response(request_id: &str, result: &str) -> String {
    format!(
        r#"{{"kind":"response","requestId":"{request_id}","status":"success","result":{result}}}"#
    )
}

fn event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":18}},"schemaVersion":{{"major":1,"minor":0}},"payload":{{"revision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

fn menu_event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"menu.action.invoked","source":"native.menu","protocolVersion":{{"major":1,"minor":18}},"schemaVersion":{{"major":1,"minor":18}},"payload":{{"menuRevision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

fn frame(message: &str) -> Vec<u8> {
    encode_json(message).expect("test message frames")
}

fn messages(written: &WriteLog) -> Vec<String> {
    let mut decoder = FrameDecoder::new();
    written
        .lock()
        .expect("test write log is available")
        .iter()
        .flat_map(|frame| decoder.push(frame).expect("client writes valid frames"))
        .collect()
}

fn request_field(message: &str, name: &str) -> Option<String> {
    JsonValue::parse(message)
        .ok()?
        .as_object()?
        .get(name)?
        .as_string()
        .map(str::to_owned)
}

fn request_protocol_minor(message: &str) -> Option<u16> {
    JsonValue::parse(message)
        .ok()?
        .as_object()?
        .get("protocolVersion")?
        .as_object()?
        .get("minor")?
        .as_u16()
}
