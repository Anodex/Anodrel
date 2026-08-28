//! Framed authenticated message processing and bounded cancellation state.

use super::*;

impl TransportSession {
    /// Accepts arbitrary chunks from one byte stream and returns complete
    /// response frames in arrival order. Any error is terminal; the OS adapter
    /// must close its stream instead of retrying or resynchronizing.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, TransportError> {
        if matches!(self.state, SessionState::Closed) {
            return Err(TransportError::SessionClosed);
        }
        let requests = match self.decoder.push(bytes) {
            Ok(requests) => requests,
            Err(error) => return self.close_with(TransportError::Wire(error)),
        };
        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            if let Some(response) = self.handle_message(&request)? {
                responses.push(self.encode_or_close(response)?);
            }
        }
        Ok(responses)
    }

    fn handle_message(&mut self, message: &str) -> Result<Option<String>, TransportError> {
        match &self.state {
            SessionState::Pending(credentials) => {
                if !matches_credentials(message, credentials) {
                    return self.close_with(TransportError::AuthenticationFailed);
                }
                self.state = SessionState::Authenticated;
                Ok(Some(
                    object([("kind", JsonValue::String(AUTHENTICATED_KIND.to_owned()))]).to_json(),
                ))
            }
            SessionState::Authenticated if has_kind(message, AUTHENTICATE_KIND) => {
                self.close_with(TransportError::AuthenticationFailed)
            }
            SessionState::Authenticated => {
                if has_kind(message, CANCELLATION_KIND) {
                    self.remember_cancellation(message)?;
                    return Ok(None);
                }
                if let Some(request) =
                    request_with_pending_cancellation(message, &mut self.pending_cancellations)
                {
                    return Ok(Some(self.host.cancelled_response(request.request_id)));
                }
                let response = self.host.handle_json(message);
                if let UiDocumentDelivery::Legacy(mailbox) = &self.ui_document_delivery
                    && let Some(snapshot) = self.host.take_ui_document_update()
                {
                    mailbox.publish(snapshot);
                }
                Ok(Some(response))
            }
            SessionState::Closed => Err(TransportError::SessionClosed),
        }
    }

    fn remember_cancellation(&mut self, message: &str) -> Result<(), TransportError> {
        let control = JsonValue::parse(message)
            .ok()
            .and_then(|value| CancellationEnvelope::from_json(value).ok())
            .filter(|control| control.protocol_version.is_supported());
        let Some(control) = control else {
            return self.close_with(TransportError::CancellationInvalid);
        };
        if self
            .pending_cancellations
            .contains(&control.cancellation_id)
        {
            return Ok(());
        }
        if self.pending_cancellations.len() == MAX_PENDING_CANCELLATIONS {
            return self.close_with(TransportError::CancellationLimitReached);
        }
        self.pending_cancellations.insert(control.cancellation_id);
        Ok(())
    }

    fn encode_or_close(&mut self, response: String) -> Result<Vec<u8>, TransportError> {
        match encode_json(&response) {
            Ok(frame) => Ok(frame),
            Err(error) => self.close_with(TransportError::Wire(error)),
        }
    }

    fn close_with<T>(&mut self, error: TransportError) -> Result<T, TransportError> {
        self.state = SessionState::Closed;
        Err(error)
    }
}

fn request_with_pending_cancellation(
    message: &str,
    pending_cancellations: &mut BTreeSet<String>,
) -> Option<RequestEnvelope> {
    let request = JsonValue::parse(message)
        .ok()
        .and_then(|value| RequestEnvelope::from_json(value).ok())?;
    request
        .cancellation_id
        .as_ref()
        .filter(|cancellation_id| pending_cancellations.remove(*cancellation_id))?;
    Some(request)
}

fn matches_credentials(message: &str, credentials: &SessionCredentials) -> bool {
    let Ok(value) = JsonValue::parse(message) else {
        return false;
    };
    let Some(fields) = value.as_object() else {
        return false;
    };
    if fields.len() != 3
        || fields.get("kind").and_then(JsonValue::as_string) != Some(AUTHENTICATE_KIND)
        || fields.get("sessionId").and_then(JsonValue::as_string)
            != Some(credentials.session_id.as_str())
    {
        return false;
    }
    let Some(token) = fields.get("token").and_then(JsonValue::as_string) else {
        return false;
    };
    is_valid_token(token) && constant_time_equals(token.as_bytes(), &credentials.token)
}

fn has_kind(message: &str, expected: &str) -> bool {
    JsonValue::parse(message)
        .ok()
        .and_then(|value| {
            value
                .as_object()
                .and_then(|fields| fields.get("kind").cloned())
        })
        .and_then(|value| value.as_string().map(str::to_owned))
        .as_deref()
        == Some(expected)
}
