//! One authenticated request/response conversation over the invited pipe.
//!
//! This module owns framing and response shape only. It never constructs a
//! pipe name, chooses a capability, or reports a native error to its caller:
//! every failure collapses into the fixture's safe stage codes.

use std::collections::VecDeque;

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_json::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use crate::pipe::{PIPE_BUFFER_BYTES, PipeClient};

/// The lowest protocol minor that carries every operation the fixture uses.
///
/// `ui.document.replace` needs 1.1, `ui.events.read` needs 1.2, and
/// `session.close` needs 1.3. Asking for the minimum keeps the fixture from
/// depending on operations it has no grant for.
const PROTOCOL_MINOR: u16 = 3;

/// A framed conversation with the host that invited this process.
pub struct FixtureSession {
    client: PipeClient,
    decoder: FrameDecoder,
    pending: VecDeque<String>,
}

impl FixtureSession {
    /// Opens the invited endpoint without sending anything.
    pub fn connect(invitation: &BootstrapInvitation) -> Option<Self> {
        let client = PipeClient::connect(invitation.pipe_name()).ok()?;
        Some(Self {
            client,
            decoder: FrameDecoder::new(),
            pending: VecDeque::new(),
        })
    }

    /// Sends the one authentication control message and awaits its result.
    pub fn authenticate(&mut self, invitation: &BootstrapInvitation) -> bool {
        let Ok(message) = invitation.authentication_message() else {
            return false;
        };
        if self.send(&message).is_none() {
            return false;
        }
        let Some(response) = self.receive() else {
            return false;
        };
        field(&response, "kind").and_then(JsonValue::as_string) == Some("session.authenticated")
    }

    /// Sends one request and returns its `result` object on success.
    ///
    /// A failure status, a malformed envelope, or a broken pipe all return
    /// `None`; the caller maps that to one documented stage code.
    pub fn request(
        &mut self,
        request_id: &str,
        operation: &str,
        payload: &str,
    ) -> Option<JsonValue> {
        let message = format!(
            r#"{{"protocolVersion":{{"major":1,"minor":{PROTOCOL_MINOR}}},"kind":"request","requestId":"{request_id}","operation":"{operation}","payload":{payload}}}"#
        );
        self.send(&message)?;
        let response = self.receive()?;
        if field(&response, "status").and_then(JsonValue::as_string) != Some("success") {
            return None;
        }
        field(&response, "result").cloned()
    }

    fn send(&mut self, message: &str) -> Option<()> {
        let frame = encode_json(message).ok()?;
        self.client.write_all(&frame).ok()
    }

    fn receive(&mut self) -> Option<JsonValue> {
        let mut buffer = [0_u8; PIPE_BUFFER_BYTES];
        loop {
            // A burst can carry more than one frame. Keeping the remainder
            // avoids discarding a response that arrived early and then blocking
            // on a read that will never produce it.
            if let Some(message) = self.pending.pop_front() {
                return JsonValue::parse(&message).ok();
            }
            let bytes_read = self.client.read(&mut buffer).ok()?;
            if bytes_read == 0 {
                return None;
            }
            self.pending
                .extend(self.decoder.push(&buffer[..bytes_read]).ok()?);
        }
    }
}

/// Reads one field from a JSON object, or `None` for any other shape.
pub fn field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(name)
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{PROTOCOL_MINOR, field};

    #[test]
    fn the_requested_minor_covers_every_operation_the_fixture_uses() {
        // Raising this without a matching grant would let the fixture reach
        // operations machine policy never approved for it.
        assert_eq!(PROTOCOL_MINOR, 3);
    }

    #[test]
    fn field_reads_only_object_members() {
        let value = JsonValue::parse(r#"{"status":"success"}"#).expect("the fixture value is JSON");
        assert_eq!(
            field(&value, "status").and_then(JsonValue::as_string),
            Some("success")
        );
        assert!(field(&value, "result").is_none());
        assert!(field(&JsonValue::Null, "status").is_none());
    }
}
