//! The closed three-operation typed UI session.

use std::io::{Read, Write};

use anodrel_client::{Client, ProtocolVersion};
use anodrel_json::JsonValue;
use anodrel_ui_document::decode;

use crate::{DocumentRevision, UiActionBatch, UiClientError};

/// The smallest protocol version that provides every typed UI-session operation.
const UI_SESSION_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(3);
/// The operation-level document input limit inside one Wire v1 message.
const MAX_SESSION_DOCUMENT_BYTES: usize = 24 * 1024;
/// The initial request sequence. Zero remains unavailable as an internal guard.
const INITIAL_REQUEST_SEQUENCE: u64 = 1;

/// One authenticated native UI session with a closed typed surface.
pub struct UiSession<Stream> {
    client: Client<Stream>,
    next_request_sequence: u64,
}

impl<Stream> UiSession<Stream>
where
    Stream: Read + Write,
{
    /// Wraps one already authenticated client conversation.
    #[must_use]
    pub fn new(client: Client<Stream>) -> Self {
        Self {
            client,
            next_request_sequence: INITIAL_REQUEST_SEQUENCE,
        }
    }

    /// Replaces this session's document with one strict v1 document.
    ///
    /// The exact document is validated locally before it is sent. The host
    /// independently validates it again and remains authoritative for the
    /// session capability and revision.
    pub fn replace_document_v1(
        &mut self,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        if document.len() > MAX_SESSION_DOCUMENT_BYTES || decode(document).is_err() {
            return Err(UiClientError::DocumentInvalid);
        }
        let result = self.request(
            "ui.document.replace",
            JsonValue::Object(
                [(
                    "document".to_owned(),
                    JsonValue::String(document.to_owned()),
                )]
                .into_iter()
                .collect(),
            ),
        )?;
        result
            .as_object()
            .and_then(|fields| fields.get("revision"))
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(DocumentRevision::parse)
    }

    /// Drains one bounded batch of host-validated semantic UI actions.
    pub fn read_actions(&mut self) -> Result<UiActionBatch, UiClientError> {
        self.request("ui.events.read", JsonValue::Object(Default::default()))
            .and_then(|result| UiActionBatch::parse(&result))
    }

    /// Requests a close of this authenticated session only.
    ///
    /// Acceptance means the host set its one-bit close signal. It does not
    /// report a window or process result.
    pub fn close(&mut self) -> Result<(), UiClientError> {
        let result = self.request("session.close", JsonValue::Object(Default::default()))?;
        if result
            .as_object()
            .and_then(|fields| fields.get("status"))
            .and_then(JsonValue::as_string)
            == Some("accepted")
        {
            Ok(())
        } else {
            Err(UiClientError::ResponseInvalid)
        }
    }

    fn request(&mut self, operation: &str, payload: JsonValue) -> Result<JsonValue, UiClientError> {
        let request_id = self.next_request_id()?;
        self.client
            .request(UI_SESSION_PROTOCOL, &request_id, operation, payload)
            .map_err(Into::into)
    }

    fn next_request_id(&mut self) -> Result<String, UiClientError> {
        let sequence = self.next_request_sequence;
        self.next_request_sequence = self
            .next_request_sequence
            .checked_add(1)
            .ok_or(UiClientError::RequestIdsExhausted)?;
        Ok(format!("anodrel-ui-{sequence}"))
    }
}

impl<Stream> std::fmt::Debug for UiSession<Stream> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UiSession")
            .field("next_request_sequence", &self.next_request_sequence)
            .finish_non_exhaustive()
    }
}
