//! End-to-end verification of the fixture child over a real Windows pipe.
//!
//! This test stands in for the host's window: it creates the same authenticated
//! interactive endpoint a registered session creates, launches the real fixture
//! executable through the real child-only bootstrap channel, publishes the one
//! semantic action the host would queue after a click, and then requires the
//! child to close its session and exit cleanly.
//!
//! It deliberately does not provision machine policy or sign anything. Machine
//! policy, locked digest revalidation, and Authenticode are covered by
//! `anodrel-windows-launch`, and the joined path needs a provisioned machine —
//! see the manual sequence in `docs/DEVELOPMENT.md`.

use std::{
    thread,
    time::{Duration, Instant},
};

use anodrel_core::{HostPolicy, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui::UiEvent;
use anodrel_ui_session::{UiDocumentMailbox, UiInputCandidate, UiInputMailbox};
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

const FIXTURE_ACTION: &str = "fixture.session.action";
const CHILD_TIMEOUT_MILLISECONDS: u32 = 30_000;
const DOCUMENT_WAIT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
fn the_fixture_completes_one_authenticated_window_round_trip() {
    let document_mailbox = UiDocumentMailbox::new();
    let input_mailbox = UiInputMailbox::new();
    let close_signal = SessionCloseSignal::default();

    // Exactly the grants the fixture's machine record declares. A missing grant
    // here would surface as the fixture's own document or event stage code.
    let policy = HostPolicy::new(
        "org.anodrel.product-fixture",
        vec![
            Capability::UiDocumentWrite,
            Capability::UiEventsRead,
            Capability::SessionClose,
        ],
        "anodrel-fixture-test-host",
    )
    .expect("the fixture test policy is valid");

    let (server, invitation) = WindowsPipeServer::create_with_session_components(
        policy,
        "fixture-test-session",
        document_mailbox.clone(),
        input_mailbox.clone(),
        close_signal.clone(),
    )
    .expect("the fixture test endpoint is created");
    let bootstrap = invitation
        .bootstrap_invitation()
        .expect("the fixture test invitation converts");

    let worker = thread::spawn(move || server.serve_one());
    let command = BootstrapCommand::new(env!("CARGO_BIN_EXE_anodrel-product-fixture"))
        .expect("the fixture executable path is a valid command");
    let child = launch(&command, &bootstrap).expect("the fixture child launches");

    // Stand in for the UI thread: take the delivered snapshot, then queue the
    // action a person would activate in the rendered window.
    let snapshot = wait_for_document(&document_mailbox, &child);
    let revision = snapshot.revision();
    assert_eq!(
        revision.value(),
        1,
        "a freshly created session must accept the fixture document as revision 1"
    );
    input_mailbox.push(UiInputCandidate::new(
        revision,
        UiEvent::ActionInvoked(
            anodrel_ui::ElementId::new(FIXTURE_ACTION).expect("the fixture action ID is valid"),
        ),
    ));

    let exit_code = child
        .wait_for_exit(CHILD_TIMEOUT_MILLISECONDS)
        .expect("the fixture child exits within its bound");
    assert_eq!(
        exit_code, 0,
        "the fixture stopped at safe stage {exit_code}; see docs/PRODUCT_FIXTURE.md"
    );

    assert!(
        close_signal.take(),
        "the fixture must request session.close before it exits"
    );
    worker
        .join()
        .expect("the fixture test pipe worker does not panic")
        .expect("the fixture test pipe worker completes");
}

#[test]
fn a_child_without_an_invitation_stops_at_its_bootstrap_stage() {
    // Launched with an empty bootstrap channel through the ordinary Windows
    // child API, the fixture must fail closed rather than searching for an
    // endpoint of its own.
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_anodrel-product-fixture"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the fixture executable starts");
    let status = child.wait().expect("the fixture child exits");

    assert_eq!(
        status.code(),
        Some(11),
        "an unreadable bootstrap record must report exactly its own stage"
    );
}

fn wait_for_document(
    mailbox: &UiDocumentMailbox,
    child: &anodrel_windows_bootstrap::LaunchedProcess,
) -> anodrel_ui_session::UiDocumentSnapshot {
    let deadline = Instant::now() + DOCUMENT_WAIT;
    while Instant::now() < deadline {
        if let Some(snapshot) = mailbox.take() {
            return snapshot;
        }
        // A child that already stopped will never deliver. Reporting its stage
        // points straight at the failing boundary instead of the timeout.
        if let Ok(exit_code) = child.wait_for_exit(0) {
            panic!(
                "the fixture stopped at safe stage {exit_code} before delivering its document; \
                 see docs/PRODUCT_FIXTURE.md"
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("the fixture did not deliver its document within its bound");
}
