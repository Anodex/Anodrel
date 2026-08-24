//! A compiled diagnostic for one authenticated document replacement.
//!
//! The child owns two fixed documents and two fixed semantic actions. It makes
//! one ordinary revision-bound action turn into one authenticated replacement,
//! then closes only after a second ordinary action. It accepts no arguments,
//! configuration, files, network input, or content.

#![deny(unsafe_op_in_unsafe_fn)]

mod document;
mod stages;

use std::{io, process::ExitCode, thread};

use anodrel_client::{Client, InteractivePollSchedule};
use anodrel_ui_client::UiSession;
use anodrel_windows_client::WindowsClientStream;

use document::{COMPLETE_ACTION, INITIAL_DOCUMENT, PREPARE_ACTION, REPLACEMENT_DOCUMENT};
use stages::Stage;

type StructureEventClient = Client<WindowsClientStream>;

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let Ok(invitation) = StructureEventClient::read_invitation(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Ok(stream) = WindowsClientStream::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    let Ok(client) = StructureEventClient::authenticate(stream, invitation) else {
        return Stage::AuthenticationRejected;
    };
    let mut session = UiSession::new(client);

    if session
        .replace_document_v1(INITIAL_DOCUMENT)
        .map_or(true, |revision| revision.value() != 1)
    {
        return Stage::InitialDocumentRejected;
    }
    match wait_for_action(&mut session, PREPARE_ACTION, 1) {
        Stage::Completed => {}
        stage => return stage,
    }
    if session
        .replace_document_v1(REPLACEMENT_DOCUMENT)
        .map_or(true, |revision| revision.value() != 2)
    {
        return Stage::ReplacementDocumentRejected;
    }
    match wait_for_action(&mut session, COMPLETE_ACTION, 2) {
        Stage::Completed => {}
        stage => return stage,
    }
    if session.close().is_err() {
        return Stage::CloseRejected;
    }
    Stage::Completed
}

/// Waits only for this stage's compiled action at this stage's revision.
///
/// A foreign action, overflow, discard, or malformed response ends the
/// diagnostic rather than allowing a different UI or revision to advance it.
fn wait_for_action(
    session: &mut UiSession<WindowsClientStream>,
    expected_action: &str,
    expected_revision: u64,
) -> Stage {
    for interval in InteractivePollSchedule::new() {
        let Ok(batch) = session.read_actions() else {
            return Stage::EventReadFailed;
        };
        if batch.dropped() != 0 || batch.discarded() != 0 {
            return Stage::EventReadFailed;
        }
        match batch.actions() {
            [] => thread::sleep(interval),
            actions
                if actions.iter().all(|action| {
                    action.action() == expected_action
                        && action.revision().value() == expected_revision
                }) =>
            {
                return Stage::Completed;
            }
            _ => return Stage::EventReadFailed,
        }
    }
    Stage::ActionNotObserved
}
