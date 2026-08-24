//! End-to-end UI Automation Invoke acceptance for one compiled UI session.
//!
//! This development diagnostic deliberately uses the existing compiled native
//! child instead of a test-only protocol. The child publishes one immutable
//! enabled action, waits for its normal revision-bound semantic event, and
//! closes the authenticated session after it receives that event.

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
};

use crate::development_ui_session::{DevelopmentUiSessionConfig, run_with_window_observer};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::new(
    "anodrel.native-uia-invoke-probe",
    "native-uia-invoke-probe-session",
    "Anodrel UI Automation Invoke Probe",
    "UI Automation Invoke probe passed.",
);
const FIXED_ACTION_ID: &str = "native.ui.complete";
const FIXED_ACTION_NAME: &str = "Complete native UI diagnostic";
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

/// Runs the one compiled native session through a real Windows Invoke call.
///
/// The selected executable is development-only and still receives exactly the
/// normal three session grants. It cannot choose an automation target or see
/// the acceptance worker's result.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_with_window_observer(
        client_path,
        CONFIGURATION,
        move |window, close| spawn(window, close, sender),
        move || await_completion(receiver),
    )
}

/// Starts the private MTA worker only after the host-created session is shown.
fn spawn(
    window: isize,
    close: SessionCloseSignal,
    completion: SyncSender<Result<(), UiAutomationError>>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-invoke-probe".to_owned())
        .spawn(move || {
            let outcome = retry_probe(window);
            if outcome.is_err() {
                // A failed probe may not leave the explicitly selected child
                // waiting for a never-arriving semantic event.
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
        Ok(Err(_)) => Err(io::Error::other(
            "UI Automation Invoke probe did not invoke the fixed authenticated action",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation Invoke probe did not report a result",
        )),
    }
}

fn retry_probe(window: isize) -> Result<(), UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match probe_once(window) {
            Ok(()) => return Ok(()),
            // The visible session begins with a host waiting view. Its fixed
            // child document appears only after the message loop consumes the
            // authenticated mailbox, so absence is retryable during startup.
            Err(
                UiAutomationError::Apartment(_)
                | UiAutomationError::Create(_)
                | UiAutomationError::Query(_)
                | UiAutomationError::NullInterface
                | UiAutomationError::UnexpectedTree,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn probe_once(window: isize) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let client = UiAutomationClient::connect()?;
    let root = client.element_from_handle(window)?;
    let action = find_fixed_action(&client, root)?.ok_or(UiAutomationError::UnexpectedTree)?;
    if !client.has_invoke_pattern(&action)? {
        return Err(UiAutomationError::UnexpectedTree);
    }
    client.invoke(&action)
}

fn find_fixed_action(
    client: &UiAutomationClient,
    element: UiAutomationElement,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let node = client.node(&element)?;
    if node.automation_id == FIXED_ACTION_ID {
        if node.name == FIXED_ACTION_NAME && node.control_type == UIA_BUTTON_CONTROL_TYPE {
            return Ok(Some(element));
        }
        return Err(UiAutomationError::UnexpectedTree);
    }
    for child in client.control_children(&element)? {
        if let Some(action) = find_fixed_action(client, child)? {
            return Ok(Some(action));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{FIXED_ACTION_ID, FIXED_ACTION_NAME, UIA_BUTTON_CONTROL_TYPE};

    #[test]
    fn invoke_probe_has_one_fixed_compiled_button_contract() {
        assert_eq!(FIXED_ACTION_ID, "native.ui.complete");
        assert_eq!(FIXED_ACTION_NAME, "Complete native UI diagnostic");
        assert_eq!(UIA_BUTTON_CONTROL_TYPE, 50_000);
    }
}
