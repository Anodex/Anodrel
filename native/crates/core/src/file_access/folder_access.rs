//! Protocol handlers for one selection-scoped folder-entry snapshot.

use super::*;

impl CoreHost {
    pub(crate) fn handle_folder_dialog_open_with_reference(
        &self,
        request: RequestEnvelope,
    ) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "dialog.open_folder.v2 does not accept a payload.",
                None,
            );
        }
        if !self.policy.has(Capability::DialogOpenFolder) {
            return self.capability_denied(request.request_id, "dialog.open_folder");
        }
        match self.folder_selections.open_folder() {
            Ok(FolderSelectionResult::Selected(selection)) => ResponseEnvelope::success(
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
                        "folderReference",
                        JsonValue::String(selection.reference().as_str().to_owned()),
                    ),
                ]),
            ),
            Ok(FolderSelectionResult::Cancelled) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cancelled".to_owned()))]),
            ),
            Err(FolderSelectionServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::DialogUnavailable,
                "folder dialog is unavailable.",
                None,
            ),
        }
    }

    pub(crate) fn handle_folder_entries_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(reference) = folder_reference_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "folder.read_entries requires one exact folder reference.",
                None,
            );
        };
        if !self.policy.has(Capability::FolderReadEntries) {
            return self.capability_denied(request.request_id, "folder.read_entries");
        }
        match self.folder_entries.read_entries(&reference) {
            Ok(entries) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("entries".to_owned())),
                    (
                        "entries",
                        JsonValue::Array(
                            entries
                                .entries()
                                .iter()
                                .map(|entry| {
                                    object([
                                        ("name", JsonValue::String(entry.name().to_owned())),
                                        (
                                            "kind",
                                            JsonValue::String(
                                                folder_entry_kind_name(entry.kind()).to_owned(),
                                            ),
                                        ),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    ("complete", JsonValue::Bool(entries.is_complete())),
                ]),
            ),
            Err(FolderEntryServiceError::Unavailable) => self.failure(
                request.request_id,
                ProtocolErrorCode::FolderUnavailable,
                "selected folder is unavailable.",
                None,
            ),
        }
    }
}

fn folder_reference_payload(value: &JsonValue) -> Option<FolderReference> {
    let fields = value.as_object()?;
    if fields.len() != 1 {
        return None;
    }
    FolderReference::new(fields.get("folderReference")?.as_string()?.to_owned()).ok()
}

fn folder_entry_kind_name(kind: anodrel_folder_access::FolderEntryKind) -> &'static str {
    match kind {
        anodrel_folder_access::FolderEntryKind::File => "file",
        anodrel_folder_access::FolderEntryKind::Directory => "directory",
        anodrel_folder_access::FolderEntryKind::Other => "other",
    }
}
