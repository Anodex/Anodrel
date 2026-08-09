//! The development-only Anodrel product-session fixture child.
//!
//! This executable exists to exercise one verified Windows product session end
//! to end: machine policy, locked digest revalidation, Authenticode publisher
//! match, private bootstrap delivery, authenticated pipe, host-owned native
//! window, one semantic action, and coordinated shutdown.
//!
//! It is not a product and not an SDK sample. It takes no arguments, reads no
//! configuration, writes no file, opens no network connection, and prints
//! nothing: its launcher redirects output to `NUL`, so the exit code is the
//! only signal. See `docs/PRODUCT_FIXTURE.md`.

#![deny(unsafe_op_in_unsafe_fn)]

mod document;
mod pipe;
mod session;
mod stages;
mod wait;

use std::{io, process::ExitCode, thread};

use anodrel_bootstrap::BootstrapInvitation;
use anodrel_json::JsonValue;

use document::{FIXTURE_ACTION, FIXTURE_DOCUMENT};
use session::{FixtureSession, field};
use stages::Stage;
use wait::PollSchedule;

/// The exact grants the fixture's machine record declares, in sorted order.
///
/// A session carrying anything else is not this fixture's session, and the
/// fixture refuses to continue rather than exercising authority it was not
/// provisioned with.
const EXPECTED_GRANTS: [&str; 3] = ["session.close", "ui.document.write", "ui.events.read"];

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let Ok(invitation) = BootstrapInvitation::read_from(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Some(mut session) = FixtureSession::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    if !session.authenticate(&invitation) {
        return Stage::AuthenticationRejected;
    }
    // The invitation has served its only purpose. Dropping it here zeroes the
    // token well before this process waits on a person.
    drop(invitation);

    if !has_exactly_its_machine_grants(&mut session) {
        return Stage::GrantsUnexpected;
    }
    if !delivers_first_document(&mut session) {
        return Stage::DocumentRejected;
    }
    match wait_for_fixture_action(&mut session) {
        Stage::Completed => {}
        stage => return stage,
    }
    if !closes_session(&mut session) {
        return Stage::CloseRejected;
    }
    Stage::Completed
}

/// Confirms the session carries exactly the grants the machine record declares.
///
/// This is the fixture's liveness check and its capability check in one. It uses
/// `platform.capabilities`, which needs no grant of its own, so the fixture can
/// verify the record's capability array reached the authenticated session
/// without requesting a diagnostics grant it has no use for.
fn has_exactly_its_machine_grants(session: &mut FixtureSession) -> bool {
    let Some(result) = session.request("fixture-grants", "platform.capabilities", "{}") else {
        return false;
    };
    let Some(JsonValue::Array(granted)) = field(&result, "grantedCapabilities") else {
        return false;
    };
    let mut granted = granted
        .iter()
        .filter_map(JsonValue::as_string)
        .collect::<Vec<_>>();
    granted.sort_unstable();
    granted == EXPECTED_GRANTS
}

/// Replaces the host's waiting screen with the fixture's one fixed document.
///
/// The first accepted replacement is always revision `1`. Anything else means
/// this session was not freshly created for this child.
fn delivers_first_document(session: &mut FixtureSession) -> bool {
    let payload = JsonValue::Object(
        [(
            "document".to_owned(),
            JsonValue::String(FIXTURE_DOCUMENT.to_owned()),
        )]
        .into_iter()
        .collect(),
    )
    .to_json();
    session
        .request("fixture-document", "ui.document.replace", &payload)
        .and_then(|result| {
            field(&result, "revision")
                .and_then(JsonValue::as_string)
                .map(str::to_owned)
        })
        .as_deref()
        == Some("1")
}

/// Polls the bounded semantic-input path until the rendered action arrives.
///
/// The wait is paced by [`PollSchedule`], so an immediate click is answered
/// within a few tens of milliseconds while an open window costs far fewer idle
/// round trips than a fixed interval would. Running out of schedule is the
/// timeout.
fn wait_for_fixture_action(session: &mut FixtureSession) -> Stage {
    for interval in PollSchedule::new() {
        let Some(result) = session.request("fixture-events", "ui.events.read", "{}") else {
            return Stage::EventReadFailed;
        };
        if number(&result, "dropped") != Some(0) || number(&result, "discarded") != Some(0) {
            return Stage::EventReadFailed;
        }
        let Some(JsonValue::Array(events)) = field(&result, "events") else {
            return Stage::EventReadFailed;
        };
        match events.first() {
            None => thread::sleep(interval),
            Some(event) if is_fixture_action(event) => return Stage::Completed,
            Some(_) => return Stage::EventReadFailed,
        }
    }
    Stage::ActionNotObserved
}

fn is_fixture_action(event: &JsonValue) -> bool {
    let Some(payload) = field(event, "payload") else {
        return false;
    };
    field(event, "eventName").and_then(JsonValue::as_string) == Some("ui.action.invoked")
        && field(payload, "action").and_then(JsonValue::as_string) == Some(FIXTURE_ACTION)
        && field(payload, "revision").and_then(JsonValue::as_string) == Some("1")
}

fn closes_session(session: &mut FixtureSession) -> bool {
    session
        .request("fixture-close", "session.close", "{}")
        .and_then(|result| {
            field(&result, "status")
                .and_then(JsonValue::as_string)
                .map(str::to_owned)
        })
        .as_deref()
        == Some("accepted")
}

fn number(value: &JsonValue, name: &str) -> Option<u16> {
    field(value, name).and_then(JsonValue::as_u16)
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::{is_fixture_action, number};

    #[test]
    fn accepts_only_the_fixture_action_at_its_own_revision() {
        let event = |action: &str, revision: &str| {
            JsonValue::parse(&format!(
                r#"{{"kind":"event","eventName":"ui.action.invoked","payload":{{"revision":"{revision}","action":"{action}"}}}}"#
            ))
            .expect("the fixture event is JSON")
        };

        assert!(is_fixture_action(&event("fixture.session.action", "1")));
        // A different action or a stale revision would mean the host rendered
        // something this fixture did not deliver.
        assert!(!is_fixture_action(&event("fixture.other", "1")));
        assert!(!is_fixture_action(&event("fixture.session.action", "2")));
        assert!(!is_fixture_action(&JsonValue::Null));
    }

    #[test]
    fn reads_bounded_counts_from_an_event_result() {
        let result = JsonValue::parse(r#"{"events":[],"dropped":0,"discarded":3}"#)
            .expect("the fixture result is JSON");
        assert_eq!(number(&result, "dropped"), Some(0));
        assert_eq!(number(&result, "discarded"), Some(3));
        assert_eq!(number(&result, "missing"), None);
    }
}
