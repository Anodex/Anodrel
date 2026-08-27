//! Protocol handling for one host-owned folder selection.

use super::*;

impl CoreHost {
    pub(crate) fn handle_folder_dialog_open(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_folder does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFolder) {
            return self.capability_denied(request.request_id, "dialog.open_folder");
        }
        match self.file_dialogs.open_folder() {
            Ok(FileDialogSelection::Folder(path)) => ResponseEnvelope::success(
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
            Ok(FileDialogSelection::Selected(_))
            | Ok(FileDialogSelection::Saved(_))
            | Ok(FileDialogSelection::Captured(_, _))
            | Ok(FileDialogSelection::CapturedSave(_, _))
            | Ok(FileDialogSelection::CapturedFolder(_, _))
            | Err(FileDialogServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "folder dialog is unavailable.",
                None,
            ),
        }
    }
}
