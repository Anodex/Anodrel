//! Focused verification for the closed Windows SDK connection boundary.

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_wire::{FrameDecoder, encode_json};

use super::{
    UiClientError, WindowFullscreenMode, WindowSize, WindowState, WindowsUiConnectionError,
    WindowsUiSession, establish_session,
};

const PIPE_NAME: &str = r"\\.\pipe\anodrel.v1.windows-sdk-test";
const SESSION_ID: &str = "windows-sdk-test-session";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[derive(Default)]
struct TestStream {
    reads: VecDeque<Vec<u8>>,
    written: Vec<u8>,
}

impl TestStream {
    fn with_reads(reads: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            reads: reads.into_iter().collect(),
            written: Vec::new(),
        }
    }
}

impl Read for TestStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let Some(next) = self.reads.pop_front() else {
            return Ok(0);
        };
        assert!(
            next.len() <= output.len(),
            "test chunk must fit the client buffer"
        );
        output[..next.len()].copy_from_slice(&next);
        Ok(next.len())
    }
}

impl Write for TestStream {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn invitation() -> BootstrapInvitation {
    BootstrapInvitation::new(PIPE_NAME, SESSION_ID, TOKEN).expect("fixed invitation is valid")
}

fn invitation_bytes() -> Vec<u8> {
    invitation().encode().expect("fixed invitation encodes")
}

#[test]
fn invalid_standard_input_stops_before_any_endpoint_is_opened() {
    let mut input = &b"not an invitation"[..];
    let result =
        establish_session::<TestStream>(&mut input, |_| panic!("must not open an endpoint"));
    assert_eq!(
        result.unwrap_err(),
        WindowsUiConnectionError::BootstrapUnavailable
    );
}

#[test]
fn endpoint_failure_is_a_closed_category() {
    let mut input = &invitation_bytes()[..];
    let result = establish_session::<TestStream>(&mut input, |_| {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "private pipe detail",
        ))
    });
    assert_eq!(
        result.unwrap_err(),
        WindowsUiConnectionError::InvitedEndpointUnavailable
    );
}

#[test]
fn authenticated_session_does_not_expose_the_transport_after_setup() {
    let response =
        encode_json(r#"{"kind":"session.authenticated"}"#).expect("fixed acknowledgement encodes");
    let mut input = &invitation_bytes()[..];
    let session = establish_session(&mut input, |_| Ok(TestStream::with_reads([response])))
        .expect("fixed invited stream authenticates");
    let debug = format!("{session:?}");
    assert!(!debug.contains(PIPE_NAME));

    let _ = FrameDecoder::new();
}

#[test]
fn refused_authentication_remains_a_closed_category() {
    let mut input = &invitation_bytes()[..];
    let result = establish_session(&mut input, |_| Ok(TestStream::default()));
    assert_eq!(
        result.unwrap_err(),
        WindowsUiConnectionError::AuthenticationUnavailable
    );
}

#[test]
fn facade_exposes_only_typed_targetless_window_controls() {
    let _set_title: fn(&mut WindowsUiSession, &str) -> Result<(), UiClientError> =
        WindowsUiSession::set_window_title;
    let _set_state: fn(&mut WindowsUiSession, WindowState) -> Result<(), UiClientError> =
        WindowsUiSession::set_window_state;
    let _request_focus: fn(&mut WindowsUiSession) -> Result<(), UiClientError> =
        WindowsUiSession::request_window_focus;
    let _set_fullscreen: fn(
        &mut WindowsUiSession,
        WindowFullscreenMode,
    ) -> Result<(), UiClientError> = WindowsUiSession::set_window_fullscreen;
    let _set_size: fn(&mut WindowsUiSession, WindowSize) -> Result<(), UiClientError> =
        WindowsUiSession::set_window_size;
}
