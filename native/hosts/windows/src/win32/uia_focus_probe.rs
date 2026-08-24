//! Fixed UI Automation focus verification for the host-owned UI Lab.
//!
//! This diagnostic is intentionally separate from the read-only property
//! probe. It opens the compiled UI Lab, asks Windows to focus its one fixed
//! field through the provider, then checks the same fixed field's fresh
//! keyboard-focus property before closing the temporary window. No application
//! can start it, choose a target, inspect its result, or acquire a UI
//! Automation object.

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

const FIXED_FIELD_ID: &str = "ui.lab.field";
const ATTACH_RETRY_WINDOW: Duration = Duration::from_secs(2);
const ATTACH_RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Verifies the UI Lab's published Windows UI Automation focus route.
///
/// This uses a distinct temporary window and host-only MTA client, so the
/// property probe remains read-only. The client can focus only the compiled
/// fixed field and reports no focus identity or result to an application.
pub fn run() -> io::Result<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_windows_after_shown(vec![ui_lab_window()], None, move |windows| {
        spawn(windows[0], sender)
    })?;
    match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => {
            println!("UI Automation focus probe passed.");
            Ok(())
        }
        Ok(Err(_)) => Err(io::Error::other(
            "UI Automation focus probe did not observe the expected fixed focus",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation focus probe did not report a result",
        )),
    }
}

/// Starts the diagnostic's private MTA worker after its UI Lab is visible.
pub(super) fn spawn(
    window: Hwnd,
    completion: SyncSender<Result<(), UiAutomationError>>,
) -> io::Result<()> {
    thread::Builder::new()
        .name("anodrel-uia-focus-probe".to_owned())
        .spawn(move || {
            let outcome = retry_probe(window);
            close_probe_window(window);
            let _ = completion.send(outcome);
        })
        .map(|_| ())
        .map_err(io::Error::other)
}

fn retry_probe(window: Hwnd) -> Result<(), UiAutomationError> {
    let deadline = Instant::now() + ATTACH_RETRY_WINDOW;
    loop {
        match probe_once(window) {
            Ok(()) => return Ok(()),
            Err(
                UiAutomationError::Apartment(_)
                | UiAutomationError::Create(_)
                | UiAutomationError::Query(_)
                | UiAutomationError::NullInterface,
            ) if Instant::now() < deadline => thread::sleep(ATTACH_RETRY_INTERVAL),
            Err(error) => return Err(error),
        }
    }
}

fn probe_once(window: Hwnd) -> Result<(), UiAutomationError> {
    let _apartment = ComApartment::initialize_mta()?;
    let client = UiAutomationClient::connect()?;
    let root = client.element_from_handle(window)?;
    let field = find_fixed_field(&client, root)?.ok_or(UiAutomationError::UnexpectedTree)?;
    client.set_focus(&field)?;
    // Fetch a fresh immutable provider publication after the UI-thread route
    // accepted the request. Windows' global focused-element query reports
    // native input focus, which is intentionally distinct from this
    // custom-drawn semantic focus state.
    let refreshed_root = client.element_from_handle(window)?;
    let refreshed_field =
        find_fixed_field(&client, refreshed_root)?.ok_or(UiAutomationError::UnexpectedTree)?;
    if client.has_keyboard_focus(&refreshed_field)? {
        Ok(())
    } else {
        Err(UiAutomationError::UnexpectedTree)
    }
}

fn find_fixed_field(
    client: &UiAutomationClient,
    element: UiAutomationElement,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    if client.node(&element)?.automation_id == FIXED_FIELD_ID {
        return Ok(Some(element));
    }
    for child in client.control_children(&element)? {
        if let Some(field) = find_fixed_field(client, child)? {
            return Ok(Some(field));
        }
    }
    Ok(None)
}

fn close_probe_window(window: Hwnd) {
    // SAFETY: `window` is the one host-created test window. A posted close
    // carries no pointer or other payload, and failed posting just means the
    // host has already torn the temporary window down.
    let _: Bool = unsafe { PostMessageW(window, WM_CLOSE, Wparam::default(), Lparam::default()) };
}

#[cfg(test)]
mod tests {
    use super::FIXED_FIELD_ID;

    #[test]
    fn focus_probe_has_one_fixed_compiled_target() {
        assert_eq!(FIXED_FIELD_ID, "ui.lab.field");
    }
}
