use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_client::Client;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use crate::{
    MAX_ACTIONS_PER_BATCH, MAX_WINDOW_ACTIONS_PER_BATCH, SessionWindowId, UiActionBatch,
    UiClientError, UiEvent, UiSession,
};

mod foundation;
mod validation;
mod window_controls;

const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.ui-client-test";
const SESSION_ID: &str = "ui-client-test-session";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"action","label":"Complete","fontSize":16,"enabled":true,"tone":"accent"}}"#;
const SCROLL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#;
const STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"status","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}"#;
const MENU: &str = r#"{"menus":[{"label":"File","items":[{"id":"template.menu.complete","label":"Complete menu template session","enabled":true,"shortcut":"Ctrl+Shift+M"}]}]}"#;
const CONTEXT_MENU: &str = r#"{"items":[{"id":"template.context.complete","label":"Complete context-menu template session","enabled":true}]}"#;

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
fn field_snapshots_use_one_exact_protocol_1_15_whole_surface_read() {
    let (mut session, written) = session_with_responses([response(
        "anodrel-ui-1",
        &field_snapshot(&[("template.form.name", "Ada")]),
    )]);

    let snapshot = session.read_fields().expect("field snapshot is typed");
    let [field] = snapshot.fields() else {
        panic!("the fixed response has one field");
    };
    assert_eq!(field.id().as_str(), "template.form.name");
    assert_eq!(field.value(), "Ada");

    let messages = messages(&written);
    assert_eq!(
        request_field(&messages[1], "operation"),
        Some("ui.fields.read".to_owned())
    );
    assert_eq!(request_protocol_minor(&messages[1]), Some(15));
    let request = JsonValue::parse(&messages[1]).expect("field read request is JSON");
    assert_eq!(
        request.as_object().and_then(|fields| fields.get("payload")),
        Some(&JsonValue::Object(Default::default()))
    );
}

#[test]
fn malformed_or_out_of_order_field_snapshots_fail_closed() {
    for result in [
        r#"{"fields":[{"id":"template.form.name","value":"Ada"},{"id":"template.form.name","value":"Grace"}]}"#,
        r#"{"fields":[{"id":"zeta","value":"Ada"},{"id":"alpha","value":"Grace"}]}"#,
        r#"{"fields":[{"id":"template.form.name","value":"one\ntwo"}]}"#,
        r#"{"fields":[{"id":"template.form.name","value":"Ada","edited":true}]}"#,
        r#"{"fields":[],"focus":"template.form.name"}"#,
    ] {
        let (mut session, _) = session_with_responses([response("anodrel-ui-1", result)]);
        assert_eq!(
            session.read_fields(),
            Err(UiClientError::ResponseInvalid),
            "invalid field response must not become a typed snapshot: {result}"
        );
    }
}

#[test]
fn native_menu_session_uses_the_fixed_protocol_1_24_surface() {
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
    assert_eq!(minors, [Some(3), Some(24), Some(24), Some(3)]);
    let menu_request = JsonValue::parse(&messages[2]).expect("menu request is JSON");
    assert_eq!(
        menu_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(MENU).expect("fixed menu is JSON"))
    );
}

#[test]
fn native_context_menu_session_uses_the_fixed_protocol_1_32_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response(
            "anodrel-ui-2",
            &context_menu_event_batch("template.context.complete", "1"),
        ),
        response("anodrel-ui-3", r#"{"status":"accepted"}"#),
    ]);

    assert_eq!(
        session
            .replace_context_menu_v1(CONTEXT_MENU)
            .expect("context menu is accepted")
            .value(),
        1
    );
    let batch = session
        .read_context_menu_actions()
        .expect("context-menu action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [action] = batch.actions() else {
        panic!("the fixed native context-menu action is preserved");
    };
    assert_eq!(action.revision().value(), 1);
    assert_eq!(action.action(), "template.context.complete");
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
            Some("menu.context.replace".to_owned()),
            Some("ui.events.read".to_owned()),
            Some("session.close".to_owned()),
        ]
    );
    let minors = messages
        .iter()
        .skip(1)
        .map(|message| request_protocol_minor(message))
        .collect::<Vec<_>>();
    assert_eq!(minors, [Some(32), Some(32), Some(3)]);
    let context_request = JsonValue::parse(&messages[1]).expect("context-menu request is JSON");
    assert_eq!(
        context_request
            .as_object()
            .and_then(|fields| fields.get("payload")),
        Some(&JsonValue::parse(CONTEXT_MENU).expect("fixed context menu is JSON"))
    );
}

#[test]
fn multi_window_session_uses_only_the_fixed_protocol_1_25_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-2", r#"{"revision":"2"}"#),
        response(
            "anodrel-ui-3",
            &window_event_batch("window-1", "template.window.complete", "2"),
        ),
        response("anodrel-ui-4", r#"{"status":"requested"}"#),
    ]);

    let secondary = session
        .open_window_v1("Template secondary", DOCUMENT)
        .expect("secondary view is accepted");
    assert_eq!(secondary.to_string(), "window-1");
    assert_eq!(
        session
            .replace_window_document_v1(secondary, DOCUMENT)
            .expect("secondary replacement is accepted")
            .value(),
        2
    );
    let batch = session
        .read_window_actions()
        .expect("tagged action batch is typed");
    assert_eq!(batch.dropped(), 0);
    assert_eq!(batch.discarded(), 0);
    let [action] = batch.actions() else {
        panic!("the one tagged action is preserved");
    };
    assert_eq!(action.window(), SessionWindowId::Secondary(secondary));
    assert_eq!(action.action().revision().value(), 2);
    assert_eq!(action.action().action(), "template.window.complete");
    session
        .close_window(secondary)
        .expect("secondary close is accepted");

    let messages = messages(&written);
    let operations = messages
        .iter()
        .skip(1)
        .map(|message| request_field(message, "operation"))
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            Some("window.open".to_owned()),
            Some("ui.document.replace.window".to_owned()),
            Some("ui.events.read.window".to_owned()),
            Some("window.close".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| { request_protocol_minor(message) == Some(25) })
    );
    let open = JsonValue::parse(&messages[1]).expect("open request is JSON");
    let open_payload = open
        .as_object()
        .and_then(|fields| fields.get("payload"))
        .expect("open has payload");
    assert_eq!(
        open_payload,
        &JsonValue::parse(&format!(
            r#"{{"title":"Template secondary","document":{}}}"#,
            JsonValue::String(DOCUMENT.to_owned()).to_json()
        ))
        .expect("expected open payload is JSON")
    );
}

#[test]
fn live_status_documents_use_only_the_explicit_protocol_1_26_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"revision":"1"}"#),
        response("anodrel-ui-2", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-3", r#"{"revision":"2"}"#),
    ]);

    assert_eq!(
        session
            .replace_document_v3(STATUS_DOCUMENT)
            .expect("primary status update is accepted")
            .value(),
        1
    );
    let secondary = session
        .open_window_v3("Status", STATUS_DOCUMENT)
        .expect("status secondary is accepted");
    assert_eq!(
        session
            .replace_window_document_v3(secondary, STATUS_DOCUMENT)
            .expect("status secondary update is accepted")
            .value(),
        2
    );

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("ui.document.replace.v3".to_owned()),
            Some("window.open.v3".to_owned()),
            Some("ui.document.replace.window.v3".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| request_protocol_minor(message) == Some(26))
    );
}

#[test]
fn secondary_scroll_documents_use_only_the_explicit_protocol_1_27_surface() {
    let (mut session, written) = session_with_responses([
        response("anodrel-ui-1", r#"{"windowId":"window-1"}"#),
        response("anodrel-ui-2", r#"{"revision":"2"}"#),
    ]);

    let secondary = session
        .open_window_v2("Scrollable notes", SCROLL_DOCUMENT)
        .expect("scroll secondary is accepted");
    assert_eq!(
        session
            .replace_window_document_v2(secondary, SCROLL_DOCUMENT)
            .expect("scroll secondary update is accepted")
            .value(),
        2
    );

    let messages = messages(&written);
    assert_eq!(
        messages
            .iter()
            .skip(1)
            .map(|message| request_field(message, "operation"))
            .collect::<Vec<_>>(),
        [
            Some("window.open.v2".to_owned()),
            Some("ui.document.replace.window.v2".to_owned()),
        ]
    );
    assert!(
        messages
            .iter()
            .skip(1)
            .all(|message| request_protocol_minor(message) == Some(27))
    );
}

#[test]
fn multi_window_action_reads_accept_one_full_group_batch() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &window_event_batch_with_count_per_window(MAX_ACTIONS_PER_BATCH),
    )]);

    let batch = session
        .read_window_actions()
        .expect("a full bounded group batch is typed");
    assert_eq!(batch.actions().len(), MAX_WINDOW_ACTIONS_PER_BATCH);
    let mut per_window = std::collections::BTreeMap::new();
    for action in batch.actions() {
        let identity = match action.window() {
            SessionWindowId::Main => "main".to_owned(),
            SessionWindowId::Secondary(window) => window.to_string(),
        };
        *per_window.entry(identity).or_insert(0_usize) += 1;
    }
    assert_eq!(per_window.len(), 4);
    assert!(
        per_window
            .values()
            .all(|count| *count == MAX_ACTIONS_PER_BATCH)
    );
}

#[test]
fn multi_window_action_reads_reject_more_than_one_view_queue_can_hold() {
    let (mut session, _) = session_with_responses([response(
        "anodrel-ui-1",
        &window_event_batch_with_count("main", MAX_ACTIONS_PER_BATCH + 1),
    )]);

    assert_eq!(
        session.read_window_actions(),
        Err(UiClientError::ResponseInvalid)
    );
}

fn session_with_responses(
    responses: impl IntoIterator<Item = String>,
) -> (UiSession<TestStream>, WriteLog) {
    let written = Arc::new(Mutex::new(Vec::new()));
    let mut frames = vec![frame(r#"{"kind":"session.authenticated"}"#)];
    frames.extend(responses.into_iter().map(|response| frame(&response)));
    let reads = frames.into_iter().flat_map(|frame| {
        frame
            .chunks(1_024)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    });
    let stream = TestStream::new(reads, Arc::clone(&written));
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

fn context_menu_event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"menu.context.action.invoked","source":"native.context_menu","protocolVersion":{{"major":1,"minor":32}},"schemaVersion":{{"major":1,"minor":32}},"payload":{{"contextMenuRevision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

fn field_snapshot(fields: &[(&str, &str)]) -> String {
    let fields = fields
        .iter()
        .map(|(id, value)| {
            format!(
                r#"{{"id":{},"value":{}}}"#,
                JsonValue::String((*id).to_owned()).to_json(),
                JsonValue::String((*value).to_owned()).to_json(),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"fields":[{fields}]}}"#)
}

fn window_event_batch(window: &str, action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":25}},"schemaVersion":{{"major":1,"minor":0}},"windowId":"{window}","payload":{{"revision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

fn window_event_batch_with_count(window: &str, count: usize) -> String {
    format!(
        r#"{{"events":[{}],"dropped":0,"discarded":0}}"#,
        window_events_with_count(window, count).join(",")
    )
}

fn window_events_with_count(window: &str, count: usize) -> Vec<String> {
    let event = format!(
        r#"{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":25}},"schemaVersion":{{"major":1,"minor":0}},"windowId":"{window}","payload":{{"revision":"1","action":"template.window.complete"}}}}"#
    );
    std::iter::repeat_n(event, count).collect()
}

fn window_event_batch_with_count_per_window(count: usize) -> String {
    let events = ["main", "window-1", "window-2", "window-3"]
        .into_iter()
        .flat_map(|window| window_events_with_count(window, count))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"events":[{events}],"dropped":0,"discarded":0}}"#)
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
