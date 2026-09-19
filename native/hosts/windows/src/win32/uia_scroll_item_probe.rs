//! Fixed UI Automation ScrollItem acceptance for the host-owned UI Lab.
//!
//! The private client selects one compiled off-screen descendant, asks Windows
//! to reveal it, and reads only a fresh publication of that same host window.
//! It does not expose a scroll target, geometry, state, or result to an
//! application, protocol, or SDK.

use std::{
    io,
    sync::mpsc::SyncSender,
    thread,
    time::{Duration, Instant},
};

use anodrel_windows_uia_client::{
    ComApartment, UiAutomationClient, UiAutomationElement, UiAutomationError,
};

use super::{
    Bool, Hwnd, Lparam, PostMessageW, WM_CLOSE, Wparam,
    launch::{run_windows_after_shown, ui_lab_window},
};

const FIXED_ITEM_ID: &str = "ui.lab.scroll.exercise-9";
const FIXED_ITEM_NAME: &str = "Scroll exercise 9";
const UIA_BUTTON_CONTROL_TYPE: i32 = 50_000;
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

/// Verifies the UI Lab's fixed real-Windows ScrollItem route.
///
/// The diagnostic starts and closes only one temporary host-owned UI Lab. Its
/// MTA client can request one compiled descendant's standard ScrollIntoView
/// operation and sees no application-provided target or result.
pub fn run() -> io::Result<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_windows_after_shown(vec![ui_lab_window()], None, move |windows| {
        spawn(windows[0], sender)
    })?;
    match receiver.recv_timeout(COMPLETION_TIMEOUT) {
        Ok(Ok(())) => {
            println!("UI Automation ScrollItem probe passed.");
            Ok(())
        }
        Ok(Err(_)) => Err(io::Error::other(
            "UI Automation ScrollItem probe did not observe its fixed reveal",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation ScrollItem probe did not report a result",
        )),
    }
}

fn spawn(window: Hwnd, completion: SyncSender<Result<(), UiAutomationError>>) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-scroll-item-probe".to_owned())
        .spawn(move || {
            let outcome = probe(window);
            close_probe_window(window);
            let _ = completion.send(outcome);
        })
        .map(|_| ())
        .map_err(io::Error::other)
}

fn probe(window: Hwnd) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let client = retry_client()?;
    let initial = retry_fixed_item(&client, window)?;
    verify_offscreen(&client, &initial)?;
    client.prepare_scroll_item(&initial)?.scroll_into_view()?;
    wait_for_revealed_item(&client, window)
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

fn retry_fixed_item(
    client: &UiAutomationClient,
    window: Hwnd,
) -> Result<UiAutomationElement, UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match client
            .element_from_handle(window)
            .and_then(|root| find_fixed_item(client, root))
            .and_then(|item| item.ok_or(UiAutomationError::UnexpectedTree))
        {
            Ok(item) => return Ok(item),
            Err(
                UiAutomationError::Query(_)
                | UiAutomationError::NullInterface
                | UiAutomationError::UnexpectedTree,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn find_fixed_item(
    client: &UiAutomationClient,
    element: UiAutomationElement,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let node = client.node(&element)?;
    if node.automation_id == FIXED_ITEM_ID {
        return (node.name == FIXED_ITEM_NAME && node.control_type == UIA_BUTTON_CONTROL_TYPE)
            .then_some(element)
            .ok_or(UiAutomationError::UnexpectedTree)
            .map(Some);
    }
    for child in client.control_children(&element)? {
        if let Some(item) = find_fixed_item(client, child)? {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

fn verify_offscreen(
    client: &UiAutomationClient,
    item: &UiAutomationElement,
) -> Result<(), UiAutomationError> {
    if client.is_offscreen(item)?
        && client.bounding_rectangle(item)?.is_empty()
        && !client.has_invoke_pattern(item)?
    {
        Ok(())
    } else {
        Err(UiAutomationError::UnexpectedTree)
    }
}

fn wait_for_revealed_item(
    client: &UiAutomationClient,
    window: Hwnd,
) -> Result<(), UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        let outcome = retry_fixed_item(client, window).and_then(|item| {
            (!client.is_offscreen(&item)?
                && !client.bounding_rectangle(&item)?.is_empty()
                && !client.has_invoke_pattern(&item)?)
            .then_some(())
            .ok_or(UiAutomationError::UnexpectedTree)
        });
        match outcome {
            Ok(()) => return Ok(()),
            Err(UiAutomationError::UnexpectedTree) if Instant::now() < deadline => {
                thread::sleep(ATTACH_RETRY_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

fn close_probe_window(window: Hwnd) {
    // SAFETY: `window` is the one host-created temporary diagnostic window. A
    // posted close contains no pointer or other payload, and failure means the
    // host already ended the test window.
    let _: Bool = unsafe { PostMessageW(window, WM_CLOSE, Wparam::default(), Lparam::default()) };
}

#[cfg(test)]
mod tests {
    use super::{FIXED_ITEM_ID, FIXED_ITEM_NAME, UIA_BUTTON_CONTROL_TYPE};

    #[test]
    fn scroll_item_probe_has_one_fixed_offscreen_descendant_contract() {
        assert_eq!(FIXED_ITEM_ID, "ui.lab.scroll.exercise-9");
        assert_eq!(FIXED_ITEM_NAME, "Scroll exercise 9");
        assert_eq!(UIA_BUTTON_CONTROL_TYPE, 50_000);
    }
}
