//! End-to-end UI Automation live-status-event acceptance for one compiled session.
//!
//! A fixed authenticated child first publishes a polite status. A private
//! direct client then attaches to the live surface, registers one temporary
//! Windows listener, and invokes one fixed action that replaces the status with
//! a changed assertive result. The child never receives listener state or event
//! delivery information.

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
    "anodrel.native-uia-live-status-event-probe",
    "native-uia-live-status-event-probe-session",
    "Anodrel UI Automation Live-Status Event Probe",
    "UI Automation live-status event probe passed.",
);
const FIXED_ROOT_ID: &str = "anodrel.surface";
const FIXED_STATUS_ID: &str = "native.live.status";
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const PREPARE_ACTION: FixedAction =
    FixedAction::new("native.live.prepare", "Publish changed live status");
const COMPLETE_ACTION: FixedAction =
    FixedAction::new("native.live.complete", "Complete live-status diagnostic");
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

/// Runs one fixed v3 session through a real Windows live-region callback.
///
/// The selected executable is development-only and receives only ordinary
/// session grants. It cannot select a source, event, listener, or outcome.
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
        .name("anodrel-uia-live-status-event-probe".to_owned())
        .spawn(move || {
            let outcome = run_worker(window);
            if outcome.is_err() {
                // A failed probe must not leave its selected child waiting for
                // an action that only this private diagnostic can trigger.
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
            "UI Automation live-status event probe did not complete its fixed sequence: {error}"
        ))),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation live-status event probe did not report a result",
        )),
    }
}

fn run_worker(window: isize) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let event_client = retry_client()?;
    let invocation = retry_fixed_invocation(&event_client, window, PREPARE_ACTION)?;
    let root = retry_fixed_root(&event_client, window)?;
    let subscription = event_client.subscribe_to_live_status_changes(&root)?;
    subscription.arm();

    invocation.invoke()?;
    subscription.wait_for_status(FIXED_STATUS_ID, EVENT_TIMEOUT)?;
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
    use super::{
        COMPLETE_ACTION, FIXED_ROOT_ID, FIXED_STATUS_ID, PREPARE_ACTION, UIA_BUTTON_CONTROL_TYPE,
    };

    #[test]
    fn live_status_event_probe_has_one_fixed_two_document_contract() {
        assert_eq!(FIXED_ROOT_ID, "anodrel.surface");
        assert_eq!(FIXED_STATUS_ID, "native.live.status");
        assert_eq!(PREPARE_ACTION.id, "native.live.prepare");
        assert_eq!(PREPARE_ACTION.name, "Publish changed live status");
        assert_eq!(COMPLETE_ACTION.id, "native.live.complete");
        assert_eq!(COMPLETE_ACTION.name, "Complete live-status diagnostic");
        assert_eq!(UIA_BUTTON_CONTROL_TYPE, 50_000);
    }
}
