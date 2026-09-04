//! Typed protocol-1.17 retained selected-output operations.

use std::io::{Read, Write};

use anodrel_client::ProtocolVersion;
use anodrel_file_access::{
    MAX_FILE_TEXT_WRITE_BYTES, SaveReference, SaveSelection, SaveSelectionResult,
};
use anodrel_file_dialog::{FileDialogFilter, MAX_FILE_DIALOG_FILTERS, SaveFilePath};
use anodrel_json::JsonValue;

use crate::{UiClientError, UiSession};

/// The first protocol version with selected-output capture and text writing.
const FILE_WRITE_PROTOCOL: ProtocolVersion = ProtocolVersion::v1(17);

impl<Stream> UiSession<Stream>
where
    Stream: Read + Write,
{
    /// Opens one host-owned save picker and captures a one-use output object.
    ///
    /// The returned path is display data only. Only the opaque reference inside
    /// a selected result can authorize [`Self::write_selected_text`], and the
    /// host consumes that reference before any retained object is mutated.
    pub fn select_save_file_v2(
        &mut self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, UiClientError> {
        let result = self.request(
            FILE_WRITE_PROTOCOL,
            "dialog.save_file.v2",
            filter_payload(filters)?,
        )?;
        parse_save_selection(&result)
    }

    /// Writes one bounded UTF-8 value through a host-retained output object.
    ///
    /// This method never accepts a path, filename, handle, offset, append flag,
    /// durability mode, or atomicity option. A successful response confirms
    /// the bounded native write sequence, not crash-safe atomic replacement.
    pub fn write_selected_text(
        &mut self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), UiClientError> {
        if text.len() > MAX_FILE_TEXT_WRITE_BYTES {
            return Err(UiClientError::FileTextInvalid);
        }
        let result = self.request(
            FILE_WRITE_PROTOCOL,
            "file.write_text",
            JsonValue::Object(
                [
                    (
                        "saveReference".to_owned(),
                        JsonValue::String(reference.as_str().to_owned()),
                    ),
                    ("text".to_owned(), JsonValue::String(text.to_owned())),
                ]
                .into_iter()
                .collect(),
            ),
        )?;
        exact_status(&result, "written")
    }
}

fn filter_payload(filters: &[FileDialogFilter]) -> Result<JsonValue, UiClientError> {
    if filters.is_empty() || filters.len() > MAX_FILE_DIALOG_FILTERS {
        return Err(UiClientError::FileDialogFiltersInvalid);
    }
    let filters = filters
        .iter()
        .map(|filter| {
            JsonValue::Object(
                [
                    (
                        "label".to_owned(),
                        JsonValue::String(filter.label().to_owned()),
                    ),
                    (
                        "extensions".to_owned(),
                        JsonValue::Array(
                            filter
                                .extensions()
                                .iter()
                                .cloned()
                                .map(JsonValue::String)
                                .collect(),
                        ),
                    ),
                ]
                .into_iter()
                .collect(),
            )
        })
        .collect();
    Ok(JsonValue::Object(
        [("filters".to_owned(), JsonValue::Array(filters))]
            .into_iter()
            .collect(),
    ))
}

fn parse_save_selection(result: &JsonValue) -> Result<SaveSelectionResult, UiClientError> {
    let fields = result.as_object().ok_or(UiClientError::ResponseInvalid)?;
    match fields.get("status").and_then(JsonValue::as_string) {
        Some("cancelled") if fields.len() == 1 => Ok(SaveSelectionResult::Cancelled),
        Some("selected") if fields.len() == 3 => {
            let path = fields
                .get("path")
                .and_then(JsonValue::as_string)
                .ok_or(UiClientError::ResponseInvalid)?;
            let reference = fields
                .get("saveReference")
                .and_then(JsonValue::as_string)
                .ok_or(UiClientError::ResponseInvalid)?;
            let path = SaveFilePath::new(path).map_err(|_| UiClientError::ResponseInvalid)?;
            let reference =
                SaveReference::new(reference).map_err(|_| UiClientError::ResponseInvalid)?;
            Ok(SaveSelectionResult::Selected(SaveSelection::new(
                path, reference,
            )))
        }
        _ => Err(UiClientError::ResponseInvalid),
    }
}

fn exact_status(result: &JsonValue, expected: &str) -> Result<(), UiClientError> {
    let fields = result.as_object().ok_or(UiClientError::ResponseInvalid)?;
    if fields.len() == 1 && fields.get("status").and_then(JsonValue::as_string) == Some(expected) {
        Ok(())
    } else {
        Err(UiClientError::ResponseInvalid)
    }
}
