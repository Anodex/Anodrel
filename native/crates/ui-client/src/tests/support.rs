//! Shared in-memory transport and fixed protocol fixtures for UI-client tests.

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_client::Client;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use crate::UiSession;

pub(super) const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"action","label":"Complete","fontSize":16,"enabled":true,"tone":"accent"}}"#;
pub(super) const SCROLL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}}"#;
pub(super) const STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"status","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}"#;
pub(super) const MENU: &str = r#"{"menus":[{"label":"File","items":[{"id":"template.menu.complete","label":"Complete menu template session","enabled":true,"shortcut":"Ctrl+Shift+M"}]}]}"#;
pub(super) const CONTEXT_MENU: &str = r#"{"items":[{"id":"template.context.complete","label":"Complete context-menu template session","enabled":true}]}"#;

const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.ui-client-test";
const SESSION_ID: &str = "ui-client-test-session";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub(super) type WriteLog = Arc<Mutex<Vec<Vec<u8>>>>;

#[derive(Debug)]
pub(super) struct TestStream {
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

pub(super) fn session_with_responses(
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

pub(super) fn response(request_id: &str, result: &str) -> String {
    format!(
        r#"{{"kind":"response","requestId":"{request_id}","status":"success","result":{result}}}"#
    )
}

pub(super) fn event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":18}},"schemaVersion":{{"major":1,"minor":0}},"payload":{{"revision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

pub(super) fn menu_event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"menu.action.invoked","source":"native.menu","protocolVersion":{{"major":1,"minor":18}},"schemaVersion":{{"major":1,"minor":18}},"payload":{{"menuRevision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

pub(super) fn context_menu_event_batch(action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"menu.context.action.invoked","source":"native.context_menu","protocolVersion":{{"major":1,"minor":32}},"schemaVersion":{{"major":1,"minor":32}},"payload":{{"contextMenuRevision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

pub(super) fn field_snapshot(fields: &[(&str, &str)]) -> String {
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

pub(super) fn window_event_batch(window: &str, action: &str, revision: &str) -> String {
    format!(
        r#"{{"events":[{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":25}},"schemaVersion":{{"major":1,"minor":0}},"windowId":"{window}","payload":{{"revision":"{revision}","action":"{action}"}}}}],"dropped":0,"discarded":0}}"#
    )
}

pub(super) fn window_event_batch_with_count(window: &str, count: usize) -> String {
    format!(
        r#"{{"events":[{}],"dropped":0,"discarded":0}}"#,
        window_events_with_count(window, count).join(",")
    )
}

pub(super) fn window_event_batch_with_count_per_window(count: usize) -> String {
    let events = ["main", "window-1", "window-2", "window-3"]
        .into_iter()
        .flat_map(|window| window_events_with_count(window, count))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"events":[{events}],"dropped":0,"discarded":0}}"#)
}

fn window_events_with_count(window: &str, count: usize) -> Vec<String> {
    let event = format!(
        r#"{{"kind":"event","eventName":"ui.action.invoked","source":"native.ui","protocolVersion":{{"major":1,"minor":25}},"schemaVersion":{{"major":1,"minor":0}},"windowId":"{window}","payload":{{"revision":"1","action":"template.window.complete"}}}}"#
    );
    std::iter::repeat_n(event, count).collect()
}

pub(super) fn messages(written: &WriteLog) -> Vec<String> {
    let mut decoder = FrameDecoder::new();
    written
        .lock()
        .expect("test write log is available")
        .iter()
        .flat_map(|frame| decoder.push(frame).expect("client writes valid frames"))
        .collect()
}

pub(super) fn request_field(message: &str, name: &str) -> Option<String> {
    JsonValue::parse(message)
        .ok()?
        .as_object()?
        .get(name)?
        .as_string()
        .map(str::to_owned)
}

pub(super) fn request_protocol_minor(message: &str) -> Option<u16> {
    JsonValue::parse(message)
        .ok()?
        .as_object()?
        .get("protocolVersion")?
        .as_object()?
        .get("minor")?
        .as_u16()
}

fn frame(message: &str) -> Vec<u8> {
    encode_json(message).expect("test message frames")
}
