//! File-picker, selected-input, and selected-output protocol handlers.
//!
//! The portable core validates only narrow protocol values and checks the
//! session policy. Native adapters own file identity capture and I/O.

mod folder;

use super::*;

impl CoreHost {
    pub(super) fn handle_file_dialog_open(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_dialogs.open_file(&filters) {
            Ok(FileDialogSelection::Selected(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Folder(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _))
            | Ok(FileDialogSelection::CapturedFolder(_, _)) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog returned an incompatible result.",
                None,
            ),
            Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_dialog_save(&self, request: RequestEnvelope) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogSaveFile) {
            return self.capability_denied(request.request_id, "dialog.save_file");
        }
        match self.file_dialogs.save_file(&filters) {
            Ok(FileDialogSelection::Saved(path)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("saved".to_owned())),
                    (
                        "path",
                        JsonValue::String(path.as_path().to_string_lossy().into_owned()),
                    ),
                ]),
            ),
            Ok(FileDialogSelection::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Folder(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _))
            | Ok(FileDialogSelection::CapturedFolder(_, _))
            | Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_dialog_open_with_reference(
        &self,
        request: RequestEnvelope,
    ) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_file.v2 filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFile) {
            return self.capability_denied(request.request_id, "dialog.open_file");
        }
        match self.file_selections.open_file(&filters) {
            Ok(FileSelectionResult::Selected(selection)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(
                            selection.path().as_path().to_string_lossy().into_owned(),
                        ),
                    ),
                    (
                        "selectionReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(FileSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(FileSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_dialog_save_with_reference(
        &self,
        request: RequestEnvelope,
    ) -> JsonValue {
        let Some(filters) = file_dialog_open_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file.v2 requires strict bounded filters.",
                None,
            );
        };
        if request.payload.to_json().len() > MAX_FILE_DIALOG_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.save_file.v2 filters exceeded the operation size limit.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogSaveFile) {
            return self.capability_denied(request.request_id, "dialog.save_file");
        }
        match self.file_save_selections.save_file(&filters) {
            Ok(SaveSelectionResult::Selected(selection)) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("selected".to_owned())),
                    (
                        "path",
                        JsonValue::String(
                            selection.path().as_path().to_string_lossy().into_owned(),
                        ),
                    ),
                    (
                        "saveReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(SaveSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(SaveSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "file dialog is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_text_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(reference) = file_selection_reference_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.read_text requires one exact selection reference.",
                None,
            );
        };
        if !self.policy.has(Capability::FileReadText) {
            return self.capability_denied(request.request_id, "file.read_text");
        }
        match self.file_text.read_text(&reference) {
            Ok(text) if text.len() <= MAX_FILE_TEXT_RESPONSE_BYTES => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("text".to_owned())),
                    ("text", JsonValue::String(text)),
                ]),
            ),
            Ok(_) | Err(FileTextServiceError::TooLarge) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected file text is too large.",
                None,
            ),
            Err(FileTextServiceError::InvalidText) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextInvalid,
                "selected file text is invalid.",
                None,
            ),
            Err(FileTextServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected file is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_text_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((reference, text)) = file_text_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.write_text requires one exact save reference and text.",
                None,
            );
        };
        if text.len() > MAX_FILE_TEXT_WRITE_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected output text is too large.",
                None,
            );
        }
        if !self.policy.has(Capability::FileWriteText) {
            return self.capability_denied(request.request_id, "file.write_text");
        }
        match self.file_text_write.write_text(&reference, text) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(FileTextWriteServiceError::TooLarge) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileTextTooLarge,
                "selected output text is too large.",
                None,
            ),
            Err(FileTextWriteServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected output is unavailable.",
                None,
            ),
        }
    }

    pub(super) fn handle_file_binary_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((reference, encoded)) = file_binary_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "file.write_binary requires one exact save reference and canonical base64url data.",
                None,
            );
        };
        if !self.policy.has(Capability::FileWriteBinary) {
            return self.capability_denied(request.request_id, "file.write_binary");
        }
        let data = match FileBinaryData::decode_base64url(encoded) {
            Ok(data) => data,
            Err(FileBinaryDataError::Invalid) => {
                self.file_binary_write.discard(&reference);
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "file.write_binary requires canonical base64url data.",
                    None,
                );
            }
            Err(FileBinaryDataError::TooLarge) => {
                self.file_binary_write.discard(&reference);
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::FileBinaryTooLarge,
                    "selected output binary data is too large.",
                    None,
                );
            }
        };
        match self.file_binary_write.write_binary(&reference, &data) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(FileBinaryWriteServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FileUnavailable,
                "selected output is unavailable.",
                None,
            ),
        }
    }
}

fn file_dialog_open_payload(value: &JsonValue) -> Option<Vec<FileDialogFilter>> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    let JsonValue::Array(filters) = fields.get("filters")? else {
        return None;
    };
    if filters.is_empty() || filters.len() > MAX_FILE_DIALOG_FILTERS {
        return None;
    }
    filters
        .iter()
        .map(|filter| {
            let fields = filter.as_object()?;
            if fields.len() != 2 {
                return None;
            }
            let label = fields.get("label")?.as_string()?.to_owned();
            let JsonValue::Array(extensions) = fields.get("extensions")? else {
                return None;
            };
            let extensions = extensions
                .iter()
                .map(|extension| extension.as_string().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?;
            FileDialogFilter::new(label, extensions).ok()
        })
        .collect()
}

fn file_selection_reference_payload(value: &JsonValue) -> Option<SelectionReference> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    SelectionReference::new(fields.get("selectionReference")?.as_string()?.to_owned()).ok()
}

fn file_text_write_payload(value: &JsonValue) -> Option<(SaveReference, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let reference =
        SaveReference::new(fields.get("saveReference")?.as_string()?.to_owned()).ok()?;
    Some((reference, fields.get("text")?.as_string()?))
}

fn file_binary_write_payload(value: &JsonValue) -> Option<(SaveReference, &str)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let reference =
        SaveReference::new(fields.get("saveReference")?.as_string()?.to_owned()).ok()?;
    Some((reference, fields.get("bytesBase64Url")?.as_string()?))
}
