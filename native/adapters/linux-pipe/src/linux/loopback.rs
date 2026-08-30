//! Private real-socket health probe for the Linux transport adapter.

use std::{
    io::{self, Read, Write},
    thread,
};

use anodrel_core::HostPolicy;
use anodrel_protocol::JsonValue;
use anodrel_wire::{FrameDecoder, encode_json};

use super::{LinuxPipeServer, SessionInvitation};

const HEALTH_REQUEST: &str = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"linux-transport-health","operation":"platform.health","payload":{}}"#;
const READ_BUFFER_BYTES: usize = 4 * 1024;

/// Runs one private authentication and `platform.health` round trip.
pub fn run_health_self_test(policy: HostPolicy) -> io::Result<()> {
    let (server, invitation) =
        LinuxPipeServer::create(policy, "linux-transport-loopback").map_err(|_| failed())?;
    let mut client = invitation.connect().map_err(|_| failed())?;
    let worker = thread::spawn(move || server.serve_one());

    let client_result = authenticated_health(&mut client, &invitation);
    drop(client);
    drop(invitation);
    let worker_result = worker.join().map_err(|_| failed())?.map_err(|_| failed());
    client_result?;
    worker_result
}

pub(super) fn authenticated_health(
    client: &mut impl ReadWrite,
    invitation: &SessionInvitation,
) -> io::Result<()> {
    write_json(
        client,
        &invitation.authentication_payload().map_err(|_| failed())?,
    )?;
    require_field(&read_json(client)?, "kind", "session.authenticated")?;
    write_json(client, HEALTH_REQUEST)?;
    let response = read_json(client)?;
    require_field(&response, "status", "success")?;
    let result = response
        .as_object()
        .and_then(|fields| fields.get("result"))
        .ok_or_else(failed)?;
    require_field(result, "status", "ready")
}

pub(super) trait ReadWrite: Read + Write {}

impl<T: Read + Write> ReadWrite for T {}

fn write_json(client: &mut impl Write, message: &str) -> io::Result<()> {
    let frame = encode_json(message).map_err(|_| failed())?;
    client.write_all(&frame).map_err(|_| failed())
}

fn read_json(client: &mut impl Read) -> io::Result<JsonValue> {
    let mut decoder = FrameDecoder::new();
    let mut buffer = [0_u8; READ_BUFFER_BYTES];
    loop {
        let bytes_read = client.read(&mut buffer).map_err(|_| failed())?;
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
    io::Error::other("private Linux IPC self-test did not complete")
}
