//! Private in-process probe for the named-pipe transport.
//!
//! This module deliberately stays below the adapter's public surface. It is
//! used by the Windows Startup Lab to prove the DACL, invitation,
//! authentication, frame decoder, and policy-bound health path without
//! creating an application-facing client API.

use std::{
    io,
    time::{Duration, Instant},
};

use anodrel_protocol::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use crate::{SessionInvitation, raw};

const HEALTH_REQUEST: &str = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"startup-lab-health","operation":"platform.health","payload":{}}"#;
#[cfg(test)]
const CANCELLED_HEALTH_REQUEST: &str = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"startup-lab-cancelled-health","operation":"platform.health","payload":{},"cancellationId":"startup-lab-stop"}"#;
#[cfg(test)]
const HEALTH_CANCELLATION: &str = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"cancel","cancellationId":"startup-lab-stop"}"#;

pub(super) fn authenticated_health(
    client: &raw::OwnedHandle,
    invitation: &SessionInvitation,
) -> io::Result<()> {
    authenticate(client, invitation)?;
    write_json(client, HEALTH_REQUEST)?;
    let health = read_json(client)?;
    require_field(&health, "status", "success")?;
    let result = health
        .as_object()
        .and_then(|fields| fields.get("result"))
        .ok_or_else(failed)?;
    require_field(result, "status", "ready")
}

#[cfg(test)]
pub(super) fn authenticated_cancelled_health(
    client: &raw::OwnedHandle,
    invitation: &SessionInvitation,
) -> io::Result<()> {
    authenticate(client, invitation)?;
    write_json(client, HEALTH_CANCELLATION)?;
    write_json(client, CANCELLED_HEALTH_REQUEST)?;
    let response = read_json(client)?;
    require_field(&response, "status", "failure")?;
    let error = response
        .as_object()
        .and_then(|fields| fields.get("error"))
        .ok_or_else(failed)?;
    require_field(error, "code", "request.cancelled")
}

pub(super) fn measure_authenticated_request(
    client: &raw::OwnedHandle,
    invitation: &SessionInvitation,
    request_frame: &[u8],
    warmup_iterations: usize,
    measured_iterations: usize,
) -> io::Result<Vec<Duration>> {
    authenticate(client, invitation)?;
    for _ in 0..warmup_iterations {
        request_response(client, request_frame)?;
    }

    let mut measurements = Vec::with_capacity(measured_iterations);
    for _ in 0..measured_iterations {
        let started = Instant::now();
        request_response(client, request_frame)?;
        measurements.push(started.elapsed());
    }
    Ok(measurements)
}

fn authenticate(client: &raw::OwnedHandle, invitation: &SessionInvitation) -> io::Result<()> {
    write_json(
        client,
        &invitation.authentication_payload().map_err(|_| failed())?,
    )?;
    let authentication = read_json(client)?;
    require_field(&authentication, "kind", "session.authenticated")
}

fn request_response(client: &raw::OwnedHandle, request_frame: &[u8]) -> io::Result<()> {
    raw::write_all(client, request_frame).map_err(|_| failed())?;
    let response = read_json(client)?;
    require_field(&response, "status", "success")
}

fn write_json(client: &raw::OwnedHandle, message: &str) -> io::Result<()> {
    let frame = encode_json(message).map_err(|_| failed())?;
    raw::write_all(client, &frame).map_err(|_| failed())
}

fn read_json(client: &raw::OwnedHandle) -> io::Result<JsonValue> {
    let mut decoder = FrameDecoder::new();
    let mut buffer = [0_u8; raw::PIPE_BUFFER_BYTES];
    loop {
        let bytes_read = raw::read(client, &mut buffer).map_err(|_| failed())?;
        if bytes_read == 0 {
            return Err(failed());
        }
        let messages = decoder.push(&buffer[..bytes_read]).map_err(|_| failed())?;
        if let Some(message) = messages.into_iter().next() {
            return JsonValue::parse(&message).map_err(|_| failed());
        }
    }
}

fn require_field(value: &JsonValue, field: &str, expected: &str) -> io::Result<()> {
    if value
        .as_object()
        .and_then(|fields| fields.get(field))
        .and_then(JsonValue::as_string)
        == Some(expected)
    {
        Ok(())
    } else {
        Err(failed())
    }
}

fn failed() -> io::Error {
    io::Error::other("private IPC self-test did not complete")
}
