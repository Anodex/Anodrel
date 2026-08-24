//! End-to-end UI Automation structure-event acceptance for one compiled session.
//!
//! The fixed development child first publishes an ordinary authenticated
//! document. A private direct client attaches to that live surface, then
//! invokes one fixed action which makes the child publish its one fixed
//! replacement document. This gives the existing host-only structure event a
//! deterministic, real-Windows cause without exposing a test hook to a child.

use std::{
    error::Error,
    io,
    sync::mpsc::{Receiver, SyncSender},
    thread,
    time::{Duration, Instant},
};

use anodrel_core::SessionCloseSignal;
use anodrel_windows_uia_client::{
    ComApartment, UiAutomationClient, UiAutomationElement, UiAutomationError,
    UiAutomationInvocation,
};

use crate::development_ui_session::{DevelopmentUiSessionConfig, run_with_window_observer};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::new(
    "anodrel.native-uia-structure-event-probe",
    "native-uia-structure-event-probe-session",
    "Anodrel UI Automation Structure Event Probe",
    "UI Automation structure-event probe passed.",
);
const FIXED_ROOT_ID: &str = "anodrel.surface";
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const PREPARE_ACTION: FixedAction =
    FixedAction::new("native.structure.prepare", "Prepare structure replacement");
const COMPLETE_ACTION: FixedAction =
    FixedAction::new("native.structure.complete", "Complete structure diagnostic");
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const EVENT_TIMEOUT: Duration = Duration::from_secs(2);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
struct FixedAction {
    id: &'static str,
    name: &'static str,
}

impl FixedAction {
    const fn new(id: &'static str, name: &'static str) -> Self {
        Self { id, name }
    }
}

/// Runs one fixed two-document session through a real Windows structure event.
///
/// The selected executable is development-only and receives only the normal
/// three session grants. It cannot choose an event target, observe listener
/// readiness, or read this diagnostic's result.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_with_window_observer(
        client_path,
        CONFIGURATION,
        move |window, close| spawn(window, close, sender),
        move || await_completion(receiver),
    )
}

/// Starts the private MTA worker after the host-created session is shown.
fn spawn(
    window: isize,
    close: SessionCloseSignal,
    completion: SyncSender<Result<(), UiAutomationError>>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-structure-event-probe".to_owned())
        .spawn(move || {
            let outcome = run_worker(window);
            if outcome.is_err() {
                // A failed probe may not leave the explicitly selected child
                // waiting for a never-arriving semantic action.
                close.request();
            }
            let _ = completion.send(outcome);
        })
        .map(|_| ())
        .map_err(io::Error::other)
}

fn await_completion(receiver: Receiver<Result<(), UiAutomationError>>) -> io::Result<()> {
    match receiver.recv_timeout(COMPLETION_TIMEOUT) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(io::Error::other(format!(
            "UI Automation structure-event probe did not complete its fixed sequence: {error}"
        ))),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation structure-event probe did not report a result",
        )),
    }
}

fn run_worker(window: isize) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let event_client = retry_client()?;
    let invocation = retry_fixed_invocation(&event_client, window, PREPARE_ACTION)?;
    let root = retry_fixed_root(&event_client, window)?;
    let subscription = event_client.subscribe_to_structure_changes(&root)?;
    subscription.arm();

    invocation.invoke()?;
    subscription.wait_for_children_invalidated(FIXED_ROOT_ID, EVENT_TIMEOUT)?;
    drop(subscription);
    drop(root);
    drop(event_client);

    let completion_client = retry_client()?;
    retry_fixed_invocation(&completion_client, window, COMPLETE_ACTION)?.invoke()
}

fn retry_client() -> Result<UiAutomationClient, UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match UiAutomationClient::connect() {
            Ok(client) => return Ok(client),
            Err(
                UiAutomationError::Create(_)
                | UiAutomationError::Query(_)
                | UiAutomationError::NullInterface,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn retry_fixed_root(
    client: &UiAutomationClient,
    window: isize,
) -> Result<UiAutomationElement, UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match fixed_root_once(client, window) {
            Ok(root) => return Ok(root),
            Err(
                UiAutomationError::Query(_)
                | UiAutomationError::NullInterface
                | UiAutomationError::UnexpectedTree,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn fixed_root_once(
    client: &UiAutomationClient,
    window: isize,
) -> Result<UiAutomationElement, UiAutomationError> {
    let root = client.element_from_handle(window)?;
    if client.node(&root)?.automation_id == FIXED_ROOT_ID {
        Ok(root)
    } else {
        Err(UiAutomationError::UnexpectedTree)
    }
}

fn retry_fixed_invocation(
    client: &UiAutomationClient,
    window: isize,
    expected: FixedAction,
) -> Result<UiAutomationInvocation, UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match client
            .element_from_handle(window)
            .and_then(|root| find_fixed_action(client, root, expected))
            .and_then(|action| action.ok_or(UiAutomationError::UnexpectedTree))
            .and_then(|action| client.prepare_invoke(&action))
        {
            Ok(invocation) => return Ok(invocation),
            Err(
                UiAutomationError::Query(_)
                | UiAutomationError::NullInterface
                | UiAutomationError::UnexpectedTree,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn find_fixed_action(
    client: &UiAutomationClient,
    element: UiAutomationElement,
    expected: FixedAction,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let node = client.node(&element)?;
    if node.automation_id == expected.id {
        if node.name == expected.name && node.control_type == UIA_BUTTON_CONTROL_TYPE {
            return Ok(Some(element));
        }
        return Err(UiAutomationError::UnexpectedTree);
    }
    for child in client.control_children(&element)? {
        if let Some(action) = find_fixed_action(client, child, expected)? {
            return Ok(Some(action));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{COMPLETE_ACTION, FIXED_ROOT_ID, PREPARE_ACTION, UIA_BUTTON_CONTROL_TYPE};

    #[test]
    fn structure_event_probe_has_one_fixed_two_document_contract() {
        assert_eq!(FIXED_ROOT_ID, "anodrel.surface");
        assert_eq!(PREPARE_ACTION.id, "native.structure.prepare");
        assert_eq!(PREPARE_ACTION.name, "Prepare structure replacement");
        assert_eq!(COMPLETE_ACTION.id, "native.structure.complete");
        assert_eq!(COMPLETE_ACTION.name, "Complete structure diagnostic");
        assert_eq!(UIA_BUTTON_CONTROL_TYPE, 50_000);
    }
}
