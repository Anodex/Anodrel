//! Focused verification for the authenticated Linux abstract Unix-socket adapter.

use std::{
    io::{Read, Write},
    thread,
    time::Duration,
};

use anodrel_core::HostPolicy;
use anodrel_protocol::Capability;
use anodrel_wire::encode_json;

use super::*;

fn policy() -> HostPolicy {
    HostPolicy::new(
        "test.application",
        vec![Capability::DiagnosticsRead],
        "test-host",
    )
    .expect("test policy is valid")
}

#[test]
fn serves_an_authenticated_health_request_over_a_real_linux_socket() {
    run_health_self_test(policy()).expect("private Linux IPC self-test succeeds");
}

#[test]
fn names_each_abstract_endpoint_independently() {
    let (_first_server, first) =
        LinuxPipeServer::create(policy(), "first").expect("first Linux endpoint creates");
    let (_second_server, second) =
        LinuxPipeServer::create(policy(), "second").expect("second Linux endpoint creates");
    assert_ne!(first.endpoint_name, second.endpoint_name);
    assert!(first.endpoint_name.starts_with(ENDPOINT_PREFIX));
    assert_eq!(first.endpoint_name.len(), ENDPOINT_PREFIX.len() + 64);
}

#[test]
fn invitation_redacts_the_authentication_token() {
    let (_server, invitation) =
        LinuxPipeServer::create(policy(), "redaction").expect("Linux endpoint creates");
    let payload = invitation
        .authentication_payload()
        .expect("invitation authenticates");
    assert!(format!("{invitation:?}").contains("<redacted>"));
    assert!(!format!("{invitation:?}").contains(&payload));
}

#[test]
fn rejects_an_invalid_first_authentication_frame() {
    let (server, invitation) =
        LinuxPipeServer::create(policy(), "invalid-auth").expect("Linux endpoint creates");
    let mut client = invitation.connect().expect("test client connects");
    let worker = thread::spawn(move || server.serve_one());
    let invalid = encode_json(r#"{"kind":"session.authenticate","sessionId":"invalid","token":"0000000000000000000000000000000000000000000000000000000000000000"}"#)
        .expect("invalid authentication frame encodes");
    client
        .write_all(&invalid)
        .expect("invalid authentication frame writes");
    client
        .set_read_timeout(Some(Duration::from_millis(250)))
        .expect("client read timeout sets");
    let mut byte = [0_u8; 1];
    assert!(matches!(client.read(&mut byte), Ok(0) | Err(_)));
    drop(client);
    assert!(worker.join().expect("worker does not panic").is_err());
}

#[test]
fn host_stop_signal_ends_a_pending_or_connected_worker() {
    let (server, _invitation) =
        LinuxPipeServer::create(policy(), "pending-stop").expect("Linux endpoint creates");
    server.stop_signal().request_stop();
    server
        .serve_one()
        .expect("stopped pending worker returns safely");

    let (server, invitation) =
        LinuxPipeServer::create(policy(), "connected-stop").expect("Linux endpoint creates");
    let client = invitation.connect().expect("test client connects");
    let stop = server.stop_signal();
    let worker = thread::spawn(move || server.serve_one());
    thread::sleep(Duration::from_millis(10));
    stop.request_stop();
    drop(client);
    worker
        .join()
        .expect("worker does not panic")
        .expect("stopped connected worker returns safely");
}

#[test]
fn direct_linux_peer_check_accepts_this_process() {
    let (server, invitation) =
        LinuxPipeServer::create(policy(), "peer-credentials").expect("Linux endpoint creates");
    let client = invitation.connect().expect("test client connects");
    let worker = thread::spawn(move || server.serve_one());
    assert!(endpoint::is_current_user_peer(&client).expect("peer credentials read"));
    drop(client);
    worker
        .join()
        .expect("worker does not panic")
        .expect("peer disconnect ends worker safely");
}

#[test]
fn peer_uid_check_denies_another_linux_user() {
    assert!(endpoint::matches_effective_uid(1_000, 1_000));
    assert!(!endpoint::matches_effective_uid(1_000, 1_001));
}
