//! Fixed UI Automation acceptance for targetless session-window controls.
//!
//! This host diagnostic drives one compiled child through its existing narrow
//! control sequence. The worker owns every automation target and observation;
//! the child receives only ordinary semantic actions and existing acknowledgement
//! responses. It is not an application window-inspection or input interface.

use std::{
    error::Error,
    io,
    sync::mpsc::{Receiver, SyncSender},
    thread,
    time::{Duration, Instant},
};

use anodrel_core::SessionCloseSignal;
use anodrel_windows_uia_client::{
    ComApartment, UiAutomationClient, UiAutomationElement, UiAutomationError, UiAutomationRect,
};

use crate::development_ui_session::{DevelopmentUiSessionConfig, run_with_window_observer};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_window_controls(
    "anodrel.native-uia-window-controls-probe",
    "native-uia-window-controls-probe-session",
    "Anodrel Window Controls UI Automation Probe",
    "UI Automation window-controls probe passed.",
);
const TITLE_ACTION: &str = "probe.controls.title-action";
const RESIZE_ACTION: &str = "probe.controls.resize-action";
const MAXIMIZE_ACTION: &str = "probe.controls.maximize-action";
const RESTORE_ACTION: &str = "probe.controls.restore-action";
const FOCUS_ACTION: &str = "probe.controls.focus-action";
const FULLSCREEN_ACTION: &str = "probe.controls.fullscreen-action";
const WINDOWED_ACTION: &str = "probe.controls.windowed-action";
const COMPLETE_ACTION: &str = "probe.controls.complete-action";
const TITLE_AFTER_PROPOSAL: &str =
    "Window controls exercised — Anodrel Window Controls UI Automation Probe";
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(6);

/// Runs a fixed compiled child through selected UI Automation interactions.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_with_window_observer(
        client_path,
        CONFIGURATION,
        move |window, close| spawn(window, close, sender),
        move || await_completion(receiver),
    )
}

fn spawn(
    window: isize,
    close: SessionCloseSignal,
    completion: SyncSender<Result<(), UiAutomationError>>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-window-controls-probe".to_owned())
        .spawn(move || {
            let outcome = retry_probe(window);
            if outcome.is_err() {
                // A failed observation must not strand the fixed child waiting
                // for an action the host diagnostic declined to deliver.
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
            "UI Automation window-controls probe did not observe its fixed sequence",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation window-controls probe did not report a result",
        )),
    }
}

fn retry_probe(window: isize) -> Result<(), UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match probe_once(window) {
            Ok(()) => return Ok(()),
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
    let initial = bounds(&client, window)?;

    invoke(&client, window, TITLE_ACTION)?;
    wait_for_name(&client, window, TITLE_AFTER_PROPOSAL)?;

    invoke(&client, window, RESIZE_ACTION)?;
    let resized = wait_for_changed_bounds(&client, window, initial)?;

    invoke(&client, window, MAXIMIZE_ACTION)?;
    wait_for_changed_bounds(&client, window, resized)?;

    invoke(&client, window, RESTORE_ACTION)?;
    wait_for_bounds(&client, window, resized)?;

    invoke(&client, window, FOCUS_ACTION)?;
    wait_for_action(&client, window, FULLSCREEN_ACTION)?;

    invoke(&client, window, FULLSCREEN_ACTION)?;
    wait_for_changed_bounds(&client, window, resized)?;

    invoke(&client, window, WINDOWED_ACTION)?;
    wait_for_bounds(&client, window, resized)?;

    invoke(&client, window, COMPLETE_ACTION)
}

fn invoke(
    client: &UiAutomationClient,
    window: isize,
    action_id: &str,
) -> Result<(), UiAutomationError> {
    let action = wait_for_action(client, window, action_id)?;
    client.invoke(&action)
}

fn wait_for_action(
    client: &UiAutomationClient,
    window: isize,
    action_id: &str,
) -> Result<UiAutomationElement, UiAutomationError> {
    wait_until(|| {
        let root = client.element_from_handle(window)?;
        let action = find_action(client, root, action_id)?;
        action.ok_or(UiAutomationError::UnexpectedTree)
    })
}

fn find_action(
    client: &UiAutomationClient,
    element: UiAutomationElement,
    action_id: &str,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let node = client.node(&element)?;
    if node.automation_id == action_id {
        if node.control_type == UIA_BUTTON_CONTROL_TYPE && client.has_invoke_pattern(&element)? {
            return Ok(Some(element));
        }
        return Err(UiAutomationError::UnexpectedTree);
    }
    for child in client.control_children(&element)? {
        if let Some(action) = find_action(client, child, action_id)? {
            return Ok(Some(action));
        }
    }
    Ok(None)
}

fn bounds(
    client: &UiAutomationClient,
    window: isize,
) -> Result<UiAutomationRect, UiAutomationError> {
    let root = client.element_from_handle(window)?;
    let bounds = client.bounding_rectangle(&root)?;
    if bounds.is_empty() {
        Err(UiAutomationError::UnexpectedTree)
    } else {
        Ok(bounds)
    }
}

fn wait_for_name(
    client: &UiAutomationClient,
    window: isize,
    expected: &str,
) -> Result<(), UiAutomationError> {
    wait_until(|| {
        let root = client.element_from_handle(window)?;
        (client.node(&root)?.name == expected)
            .then_some(())
            .ok_or(UiAutomationError::UnexpectedTree)
    })
}

fn wait_for_changed_bounds(
    client: &UiAutomationClient,
    window: isize,
    prior: UiAutomationRect,
) -> Result<UiAutomationRect, UiAutomationError> {
    wait_until(|| {
        let current = bounds(client, window)?;
        (current != prior)
            .then_some(current)
            .ok_or(UiAutomationError::UnexpectedTree)
    })
}

fn wait_for_bounds(
    client: &UiAutomationClient,
    window: isize,
    expected: UiAutomationRect,
) -> Result<(), UiAutomationError> {
    wait_until(|| {
        (bounds(client, window)? == expected)
            .then_some(())
            .ok_or(UiAutomationError::UnexpectedTree)
    })
}

fn wait_until<T>(
    mut observation: impl FnMut() -> Result<T, UiAutomationError>,
) -> Result<T, UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match observation() {
            Ok(value) => return Ok(value),
            Err(UiAutomationError::UnexpectedTree) if Instant::now() < deadline => {
                thread::sleep(ATTACH_RETRY_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{COMPLETE_ACTION, TITLE_AFTER_PROPOSAL, UIA_BUTTON_CONTROL_TYPE};

    #[test]
    fn probe_uses_only_its_fixed_action_and_caption_contract() {
        assert_eq!(COMPLETE_ACTION, "probe.controls.complete-action");
        assert_eq!(UIA_BUTTON_CONTROL_TYPE, 50_000);
        assert!(TITLE_AFTER_PROPOSAL.ends_with("UI Automation Probe"));
    }
}
