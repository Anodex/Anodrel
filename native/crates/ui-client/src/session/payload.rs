//! Strict UI-document validation and closed protocol payload construction.

use anodrel_json::JsonValue;
use anodrel_ui_document::{decode, decode_v2, decode_v3};

use crate::{DocumentRevision, SecondaryWindowId, UiClientError};

/// The operation-level document input limit inside one Wire v1 message.
const MAX_SESSION_DOCUMENT_BYTES: usize = 24 * 1024;

pub(super) fn validate_document(document: &str) -> Result<(), UiClientError> {
    if document.len() > MAX_SESSION_DOCUMENT_BYTES || decode(document).is_err() {
        Err(UiClientError::DocumentInvalid)
    } else {
        Ok(())
    }
}

pub(super) fn validate_document_v2(document: &str) -> Result<(), UiClientError> {
    if document.len() > MAX_SESSION_DOCUMENT_BYTES || decode_v2(document).is_err() {
        Err(UiClientError::DocumentInvalid)
    } else {
        Ok(())
    }
}

pub(super) fn validate_document_v3(document: &str) -> Result<(), UiClientError> {
    if document.len() > MAX_SESSION_DOCUMENT_BYTES || decode_v3(document).is_err() {
        Err(UiClientError::DocumentInvalid)
    } else {
        Ok(())
    }
}

pub(super) fn document_payload(document: &str) -> JsonValue {
    JsonValue::Object(
        [(
            "document".to_owned(),
            JsonValue::String(document.to_owned()),
        )]
        .into_iter()
        .collect(),
    )
}

pub(super) fn window_document_payload(title: &str, document: &str) -> JsonValue {
    JsonValue::Object(
        [
            ("title".to_owned(), JsonValue::String(title.to_owned())),
            (
                "document".to_owned(),
                JsonValue::String(document.to_owned()),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

pub(super) fn targeted_document_payload(window: SecondaryWindowId, document: &str) -> JsonValue {
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
    )
}

pub(super) fn parse_document_revision(
    result: &JsonValue,
) -> Result<DocumentRevision, UiClientError> {
    result
        .as_object()
        .and_then(|fields| fields.get("revision"))
        .and_then(JsonValue::as_string)
        .ok_or(UiClientError::ResponseInvalid)
        .and_then(DocumentRevision::parse)
}
