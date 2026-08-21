//! A compiled diagnostic for one interactive native Anodrel session.
//!
//! The child owns one fixed document and waits for its one fixed semantic
//! action. It proves authenticated document delivery, action pull, and
//! session-close behaviour without Node.js, a webview, or machine trust.
//! It accepts no arguments, configuration, files, network input, or content.

#![deny(unsafe_op_in_unsafe_fn)]

mod document;
mod stages;

use std::{io, process::ExitCode, thread};

use anodrel_client::{Client, InteractivePollSchedule};
use anodrel_ui_client::UiSession;
use anodrel_windows_client::WindowsClientStream;

use document::{NATIVE_UI_ACTION, NATIVE_UI_DOCUMENT};
use stages::Stage;

type NativeUiClient = Client<WindowsClientStream>;

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let Ok(invitation) = NativeUiClient::read_invitation(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Ok(stream) = WindowsClientStream::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    let Ok(client) = NativeUiClient::authenticate(stream, invitation) else {
        return Stage::AuthenticationRejected;
    };
    let mut session = UiSession::new(client);

    let Ok(revision) = session.replace_document_v1(NATIVE_UI_DOCUMENT) else {
        return Stage::DocumentRejected;
    };
    if revision.value() != 1 {
        return Stage::DocumentRejected;
    }
    match wait_for_native_ui_action(&mut session) {
        Stage::Completed => {}
        stage => return stage,
    }
    if session.close().is_err() {
        return Stage::CloseRejected;
    }
    Stage::Completed
}

/// Polls the host-validated event surface until the one rendered action is
/// returned. A foreign action, overflow, discard, or malformed response ends
/// the diagnostic rather than letting it treat another UI as its own.
fn wait_for_native_ui_action(session: &mut UiSession<WindowsClientStream>) -> Stage {
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
                    action.action() == NATIVE_UI_ACTION && action.revision().value() == 1
                }) =>
            {
                return Stage::Completed;
            }
            _ => return Stage::EventReadFailed,
        }
    }
    Stage::ActionNotObserved
}
