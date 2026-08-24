//! Focused verification for the Windows authenticated named-pipe adapter.

use std::time::{SystemTime, UNIX_EPOCH};

use anodrel_application::ApplicationManifest;
use anodrel_core::HostPolicy;
use anodrel_protocol::{Capability, JsonValue};
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_wire::{FrameDecoder, encode_json};

use super::*;

fn write_json(client: &raw::OwnedHandle, message: &str) {
    raw::write_all(client, &encode_json(message).expect("test request encodes"))
        .expect("test request writes");
}

fn read_json(client: &raw::OwnedHandle) -> JsonValue {
    let mut decoder = FrameDecoder::new();
    let mut buffer = [0_u8; PIPE_BUFFER_BYTES];
    loop {
        let count = raw::read(client, &mut buffer).expect("test response reads");
        let messages = decoder
            .push(&buffer[..count])
            .expect("test response frame decodes");
        if let Some(message) = messages.into_iter().next() {
            return JsonValue::parse(&message).expect("test response is JSON");
        }
    }
}

#[test]
fn serves_an_authenticated_health_request_over_a_real_windows_pipe() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    run_health_self_test(policy).expect("private IPC self-test succeeds");
}

#[test]
fn cancels_a_not_started_request_over_a_real_windows_pipe() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    run_cancellation_self_test(policy).expect("private cancellation self-test succeeds");
}

#[test]
fn converts_a_pipe_invitation_into_a_private_bootstrap_record() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    let (_server, invitation) =
        WindowsPipeServer::create(policy, "test-session").expect("pipe server creates");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("bootstrap invitation is valid");
    assert_eq!(bootstrap.pipe_name(), invitation.pipe_name());
    assert_eq!(bootstrap.session_id(), invitation.session_id());
}

#[test]
fn host_stop_signal_prevents_a_pending_server_from_accepting_a_client() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    let (server, _invitation) =
        WindowsPipeServer::create(policy, "stopped-session").expect("pipe server creates");
    server.stop_signal().request_stop();
    server
        .serve_one()
        .expect("a stopped pending server returns safely");
}

#[test]
fn host_stop_signal_ends_a_connected_pipe_worker() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    let (server, invitation) =
        WindowsPipeServer::create(policy, "connected-stop-session").expect("pipe server creates");
    let client =
        raw::connect_client(&wide_null(invitation.pipe_name())).expect("test client connects");
    let stop = server.stop_signal();
    let worker = thread::spawn(move || server.serve_one());

    thread::sleep(Duration::from_millis(10));
    stop.request_stop();
    drop(client);

    worker
        .join()
        .expect("stopped pipe worker does not panic")
        .expect("stopped pipe worker returns safely");
}

#[test]
fn measures_authenticated_health_round_trips_over_a_real_windows_pipe() {
    let policy = HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid");
    let request = r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"loopback-health","operation":"platform.health","payload":{}}"#;

    let measurements = measure_loopback_request(policy, request, 1, 2)
        .expect("private loopback measurement succeeds");
    assert_eq!(measurements.len(), 2);
}

#[test]
fn routes_credential_requests_over_a_real_authenticated_windows_pipe() {
    let policy = HostPolicy::new(
        "anodrel.sample",
        vec![
            Capability::CredentialRead,
            Capability::CredentialWrite,
            Capability::CredentialDelete,
        ],
        "test-host",
    )
    .expect("test policy is valid");
    let credential_name = format!(
        "pipe-credential-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after the epoch")
            .as_nanos()
    );
    let identity = ApplicationManifest::parse(
        r#"{"manifestVersion":{"major":1,"minor":0},"applicationId":"anodrel.sample","displayName":"Anodrel Sample","content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}}"#,
    )
    .expect("test manifest is valid")
    .identity()
    .clone();
    let (server, invitation) = WindowsPipeServer::create_with_credential_service(
        policy,
        "credential-test-session",
        WindowsCredentialService::new(identity),
    )
    .expect("credential pipe server creates");
    let name = wide_null(invitation.pipe_name());
    let server_thread = std::thread::spawn(move || server.serve_one());
    let client = raw::connect_client(&name).expect("test client connects");

    write_json(
        &client,
        &invitation
            .authentication_payload()
            .expect("test invitation authenticates"),
    );
    assert_eq!(
        read_json(&client)
            .as_object()
            .and_then(|fields| fields.get("kind"))
            .and_then(JsonValue::as_string),
        Some("session.authenticated")
    );
    write_json(
        &client,
        &format!(
            r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-write","operation":"credential.write","payload":{{"name":"{credential_name}","secret":"00aaff"}}}}"#
        ),
    );
    assert_eq!(
        read_json(&client)
            .as_object()
            .and_then(|fields| fields.get("status"))
            .and_then(JsonValue::as_string),
        Some("success")
    );
    write_json(
        &client,
        &format!(
            r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-read","operation":"credential.read","payload":{{"name":"{credential_name}"}}}}"#
        ),
    );
    let response = read_json(&client);
    let result = response
        .as_object()
        .and_then(|fields| fields.get("result"))
        .and_then(JsonValue::as_object)
        .expect("read response has a result");
    assert_eq!(
        result.get("secret").and_then(JsonValue::as_string),
        Some("00aaff")
    );
    write_json(
        &client,
        &format!(
            r#"{{"protocolVersion":{{"major":1,"minor":12}},"kind":"request","requestId":"credential-delete","operation":"credential.delete","payload":{{"name":"{credential_name}"}}}}"#
        ),
    );
    assert_eq!(
        read_json(&client)
            .as_object()
            .and_then(|fields| fields.get("status"))
            .and_then(JsonValue::as_string),
        Some("success")
    );
    drop(client);
    server_thread
        .join()
        .expect("test pipe worker does not panic")
        .expect("test pipe worker completes");
}
