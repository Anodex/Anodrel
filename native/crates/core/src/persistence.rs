//! Persistent application-state and credential handlers.
//!
//! The host exposes only bounded application state and exact credential names
//! through explicit capabilities. Native storage locations and credential-store
//! details remain behind injected services.

use super::*;

impl CoreHost {
    pub(super) fn handle_storage_read(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.read requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateRead) {
            return self.capability_denied(request.request_id, "storage.state.read");
        }
        match self.storage.read() {
            Ok(StorageRead::Absent) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("absent".to_owned()))]),
            ),
            Ok(StorageRead::Snapshot(snapshot))
                if snapshot.as_str().len() <= MAX_STORAGE_SNAPSHOT_REQUEST_BYTES =>
            {
                ResponseEnvelope::success(
                    request.request_id,
                    &self.policy.host_name,
                    object([
                        ("status", JsonValue::String("snapshot".to_owned())),
                        ("snapshot", JsonValue::String(snapshot.as_str().to_owned())),
                    ]),
                )
            }
            Ok(StorageRead::Snapshot(_)) | Err(StorageServiceError::StoredSnapshotTooLarge) => self
                .storage_failure(
                    request.request_id,
                    StorageServiceError::StoredSnapshotTooLarge,
                ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    pub(super) fn handle_storage_replace(&self, request: RequestEnvelope) -> JsonValue {
        let Some(snapshot) = storage_replace_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.replace requires one exact snapshot.",
                None,
            );
        };
        if snapshot.len() > MAX_STORAGE_SNAPSHOT_REQUEST_BYTES {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage snapshot is too large.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateReplace) {
            return self.capability_denied(request.request_id, "storage.state.replace");
        }
        let snapshot = match StorageSnapshot::new(snapshot.to_owned()) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return self.failure(
                    request.request_id,
                    ProtocolErrorCode::RequestPayloadInvalid,
                    "storage snapshot is too large.",
                    None,
                );
            }
        };
        match self.storage.replace(&snapshot) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("replaced".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    pub(super) fn handle_storage_clear(&self, request: RequestEnvelope) -> JsonValue {
        if !is_empty_object(&request.payload) {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "storage.state.clear requires an empty payload.",
                None,
            );
        }
        if !self.policy.has(Capability::StorageStateClear) {
            return self.capability_denied(request.request_id, "storage.state.clear");
        }
        match self.storage.clear() {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("cleared".to_owned()))]),
            ),
            Err(error) => self.storage_failure(request.request_id, error),
        }
    }

    pub(super) fn handle_credential_read(&self, request: RequestEnvelope) -> JsonValue {
        let Some(name) = credential_name_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.read requires one exact credential name.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialRead) {
            return self.capability_denied(request.request_id, "credential.read");
        }
        match self.credentials.read(&name) {
            Ok(secret) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([
                    ("status", JsonValue::String("found".to_owned())),
                    ("secret", JsonValue::String(secret.to_lower_hex())),
                ]),
            ),
            Err(CredentialServiceError::NotFound) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("not_found".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    pub(super) fn handle_credential_write(&self, request: RequestEnvelope) -> JsonValue {
        let Some((name, secret)) = credential_write_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.write requires one exact credential name and canonical secret.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialWrite) {
            return self.capability_denied(request.request_id, "credential.write");
        }
        match self.credentials.write(&name, &secret) {
            Ok(()) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("written".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    pub(super) fn handle_credential_delete(&self, request: RequestEnvelope) -> JsonValue {
        let Some(name) = credential_name_payload(&request.payload) else {
            return self.failure(
                request.request_id,
                ProtocolErrorCode::RequestPayloadInvalid,
                "credential.delete requires one exact credential name.",
                None,
            );
        };
        if !self.policy.has(Capability::CredentialDelete) {
            return self.capability_denied(request.request_id, "credential.delete");
        }
        match self.credentials.delete(&name) {
            Ok(true) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("deleted".to_owned()))]),
            ),
            Ok(false) | Err(CredentialServiceError::NotFound) => ResponseEnvelope::success(
                request.request_id,
                &self.policy.host_name,
                object([("status", JsonValue::String("not_found".to_owned()))]),
            ),
            Err(error) => self.credential_failure(request.request_id, error),
        }
    }

    fn storage_failure(&self, request_id: String, error: StorageServiceError) -> JsonValue {
        let (code, message) = match error {
            StorageServiceError::Unavailable => (
                ProtocolErrorCode::StorageUnavailable,
                "application state is unavailable.",
            ),
            StorageServiceError::StoredSnapshotInvalid => (
                ProtocolErrorCode::StorageSnapshotInvalid,
                "stored application state is invalid.",
            ),
            StorageServiceError::StoredSnapshotTooLarge => (
                ProtocolErrorCode::StorageSnapshotTooLarge,
                "stored application state is too large.",
            ),
        };
        self.failure(request_id, code, message, None)
    }

    fn credential_failure(&self, request_id: String, error: CredentialServiceError) -> JsonValue {
        let (code, message) = match error {
            CredentialServiceError::NotFound => (
                ProtocolErrorCode::CredentialUnavailable,
                "credential service is unavailable.",
            ),
            CredentialServiceError::AccessDenied => (
                ProtocolErrorCode::CredentialAccessDenied,
                "credential access is denied.",
            ),
            CredentialServiceError::Unavailable => (
                ProtocolErrorCode::CredentialUnavailable,
                "credential service is unavailable.",
            ),
            CredentialServiceError::StoredSecretInvalid => (
                ProtocolErrorCode::CredentialStoredSecretInvalid,
                "stored credential is invalid.",
            ),
        };
        self.failure(request_id, code, message, None)
    }
}

fn storage_replace_payload(value: &JsonValue) -> Option<&str> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("snapshot"))
        .flatten()
        .and_then(JsonValue::as_string)
}

fn credential_name_payload(value: &JsonValue) -> Option<CredentialName> {
    let fields = value.as_object()?;
    (fields.len() == 1)
        .then(|| fields.get("name"))
        .flatten()
        .and_then(JsonValue::as_string)
        .and_then(|name| CredentialName::parse(name).ok())
}

fn credential_write_payload(value: &JsonValue) -> Option<(CredentialName, Secret)> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let name = CredentialName::parse(fields.get("name")?.as_string()?).ok()?;
    let secret = Secret::from_lower_hex(fields.get("secret")?.as_string()?).ok()?;
    Some((name, secret))
}
