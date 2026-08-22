//! The closed three-operation typed UI session.

use std::io::{Read, Write};

use anodrel_client::{Client, ProtocolVersion};
use anodrel_json::JsonValue;
use anodrel_ui_document::decode;
use anodrel_window::WindowTitleProposal;

use crate::{
    DocumentRevision, MenuRevision, SecondaryWindowId, UiActionBatch, UiClientError, UiEventBatch,
    UiFieldSnapshot, WindowUiActionBatch, menu_model::decode_menu_model,
};

/// The smallest protocol version that provides every typed UI-session operation.
const UI_DOCUMENT_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(3);
/// The first protocol version with explicit whole-surface field snapshots.
const UI_FIELD_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(15);
/// The first protocol version that also carries canonical local menu shortcuts.
const UI_MENU_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(24);
/// The first protocol version with bounded session-owned secondary views.
const UI_MULTI_WINDOW_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(25);
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
        validate_document(document)?;
        let result = self.request(
            UI_DOCUMENT_PROTOCOL,
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
        parse_document_revision(&result)
    }

    /// Drains one bounded batch of host-validated semantic UI actions.
    pub fn read_actions(&mut self) -> Result<UiActionBatch, UiClientError> {
        self.request(
            UI_DOCUMENT_PROTOCOL,
            "ui.events.read",
            JsonValue::Object(Default::default()),
        )
        .and_then(|result| UiActionBatch::parse(&result))
    }

    /// Reads every current field value on this authenticated session surface.
    ///
    /// The request has no field selector and returns no typing metadata. The
    /// host owns input, focus, caret, selection, and history; this method
    /// receives only one explicit whole-surface snapshot.
    pub fn read_fields(&mut self) -> Result<UiFieldSnapshot, UiClientError> {
        self.request(
            UI_FIELD_PROTOCOL,
            "ui.fields.read",
            JsonValue::Object(Default::default()),
        )
        .and_then(|result| UiFieldSnapshot::parse(&result))
    }

    /// Replaces this session's complete strict native menu model.
    ///
    /// The model is locally decoded before the request is sent. The host still
    /// independently validates the model, capability, revision, and native
    /// attachment on its owning UI thread.
    pub fn replace_menu_v1(&mut self, menu: &str) -> Result<MenuRevision, UiClientError> {
        let result = self.request(UI_MENU_PROTOCOL, "menu.replace", decode_menu_model(menu)?)?;
        result
            .as_object()
            .and_then(|fields| fields.get("revision"))
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(MenuRevision::parse)
    }

    /// Drains one bounded batch of every currently documented semantic event.
    ///
    /// Unlike [`Self::read_actions`], this method accepts both document and
    /// native-menu events and therefore requires a host compatible with
    /// Protocol 1.24.
    pub fn read_events(&mut self) -> Result<UiEventBatch, UiClientError> {
        self.request(
            UI_MENU_PROTOCOL,
            "ui.events.read",
            JsonValue::Object(Default::default()),
        )
        .and_then(|result| UiEventBatch::parse(&result))
    }

    /// Opens one bounded secondary view with a strict v1 document.
    ///
    /// The host remains the authority for the session capability, native
    /// window lifecycle, caption composition, and private handle mapping. A
    /// returned identity means only that it created and registered a logical
    /// secondary view for this authenticated session.
    pub fn open_window_v1(
        &mut self,
        title: &str,
        document: &str,
    ) -> Result<SecondaryWindowId, UiClientError> {
        validate_document(document)?;
        let title =
            WindowTitleProposal::new(title).map_err(|_| UiClientError::WindowTitleInvalid)?;
        let result = self.request(
            UI_MULTI_WINDOW_PROTOCOL,
            "window.open",
            JsonValue::Object(
                [
                    (
                        "title".to_owned(),
                        JsonValue::String(title.as_str().to_owned()),
                    ),
                    (
                        "document".to_owned(),
                        JsonValue::String(document.to_owned()),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )?;
        result
            .as_object()
            .and_then(|fields| fields.get("windowId"))
            .and_then(JsonValue::as_string)
            .ok_or(UiClientError::ResponseInvalid)
            .and_then(SecondaryWindowId::parse_response)
    }

    /// Replaces a strict v1 document in one secondary view this facade opened.
    ///
    /// `SecondaryWindowId` has no primary variant, so the ordinary primary
    /// document method remains the only way this facade addresses `main`.
    pub fn replace_window_document_v1(
        &mut self,
        window: SecondaryWindowId,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        validate_document(document)?;
        let result = self.request(
            UI_MULTI_WINDOW_PROTOCOL,
            "ui.document.replace.window",
            JsonValue::Object(
                [
                    (
                        "windowId".to_owned(),
                        JsonValue::String(window.protocol_string()),
                    ),
                    (
                        "document".to_owned(),
                        JsonValue::String(document.to_owned()),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        )?;
        parse_document_revision(&result)
    }

    /// Drains bounded revision-checked document actions from all session views.
    ///
    /// Each action includes only its logical source tag. The host makes no
    /// cross-view timing guarantee, and the client rejects menu or malformed
    /// event shapes rather than discarding them.
    pub fn read_window_actions(&mut self) -> Result<WindowUiActionBatch, UiClientError> {
        self.request(
            UI_MULTI_WINDOW_PROTOCOL,
            "ui.events.read.window",
            JsonValue::Object(Default::default()),
        )
        .and_then(|result| WindowUiActionBatch::parse(&result))
    }

    /// Requests host-owned close of one previously opened secondary view.
    ///
    /// A successful response confirms only that the host accepted the request;
    /// it never reports native destruction, visibility, geometry, or a close
    /// reason. The primary view cannot be expressed here.
    pub fn close_window(&mut self, window: SecondaryWindowId) -> Result<(), UiClientError> {
        let result = self.request(
            UI_MULTI_WINDOW_PROTOCOL,
            "window.close",
            JsonValue::Object(
                [(
                    "windowId".to_owned(),
                    JsonValue::String(window.protocol_string()),
                )]
                .into_iter()
                .collect(),
            ),
        )?;
        if result
            .as_object()
            .and_then(|fields| fields.get("status"))
            .and_then(JsonValue::as_string)
            == Some("requested")
        {
            Ok(())
        } else {
            Err(UiClientError::ResponseInvalid)
        }
    }

    /// Requests a close of this authenticated session only.
    ///
    /// Acceptance means the host set its one-bit close signal. It does not
    /// report a window or process result.
    pub fn close(&mut self) -> Result<(), UiClientError> {
        let result = self.request(
            UI_DOCUMENT_PROTOCOL,
            "session.close",
            JsonValue::Object(Default::default()),
        )?;
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

    fn request(
        &mut self,
        version: ProtocolVersion,
        operation: &str,
        payload: JsonValue,
    ) -> Result<JsonValue, UiClientError> {
        let request_id = self.next_request_id()?;
        self.client
            .request(version, &request_id, operation, payload)
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

fn validate_document(document: &str) -> Result<(), UiClientError> {
    if document.len() > MAX_SESSION_DOCUMENT_BYTES || decode(document).is_err() {
        Err(UiClientError::DocumentInvalid)
    } else {
        Ok(())
    }
}

fn parse_document_revision(result: &JsonValue) -> Result<DocumentRevision, UiClientError> {
    result
        .as_object()
        .and_then(|fields| fields.get("revision"))
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)
        .and_then(DocumentRevision::parse)
}

impl<Stream> std::fmt::Debug for UiSession<Stream> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UiSession")
            .field("next_request_sequence", &self.next_request_sequence)
            .finish_non_exhaustive()
    }
}
