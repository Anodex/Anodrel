//! Private child, transport, and host-window lifecycle for development sessions.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_ui_session::UiWindowGroup;
use anodrel_window::WindowTitleProposal;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

use super::config::DevelopmentUiSessionConfig;
use crate::session_ui::DevelopmentSessionUi;

const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DEVELOPMENT_STOP_CODE: u32 = 0xA11D;
const HOST_NAME: &str = "anodrel-windows-host";

/// Runs one explicitly selected compiled child through a host-owned UI session.
///
/// The executable is unverified development code by design. Its output is
/// discarded, and the route is separate from the signed product-session path.
pub(crate) fn run(
    client_path: &str,
    config: DevelopmentUiSessionConfig,
) -> Result<(), Box<dyn Error>> {
    run_with_window_observer(client_path, config, |_, _| Ok(()), || Ok(()))
}

/// Runs one development session with a host-private visible-window observer.
///
/// Both callbacks are internal verification seams. The shown callback receives
/// only the newly-created host window and that session's close signal; neither
/// can be supplied by a child, protocol request, or SDK caller. The completion
/// callback runs after the session window closes and before the child is
/// allowed to outlive a failed observation.
pub(crate) fn run_with_window_observer<F, G>(
    client_path: &str,
    config: DevelopmentUiSessionConfig,
    after_shown: F,
    after_closed: G,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(isize, SessionCloseSignal) -> io::Result<()>,
    G: FnOnce() -> io::Result<()>,
{
    // Validate the selected command before a worker is allowed to wait for a
    // process that cannot start.
    let command = BootstrapCommand::new(client_path)?;
    let ui = DevelopmentSessionUi::new();
    let policy = HostPolicy::new(config.application_id, config.grants().to_vec(), HOST_NAME)?;
    let (server, invitation, window_group) = if config.supports_multi_window() {
        let window_group = UiWindowGroup::<WindowTitleProposal>::with_primary_resources(
            ui.document.clone(),
            ui.input.clone(),
        );
        let (server, invitation) =
            WindowsPipeServer::create_with_session_window_group_and_service_bundle(
                policy,
                config.session_id,
                window_group.clone(),
                ui.close.clone(),
                HostServices::unavailable(),
            )?;
        (server, invitation, Some(window_group))
    } else if config.supports_menu() {
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                HostServices::unavailable().with_menu(ui.menu.clone()),
            )?;
        (server, invitation, None)
    } else if config.supports_context_menu() {
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                HostServices::unavailable().with_context_menu(ui.context_menu.clone()),
            )?;
        (server, invitation, None)
    } else if config.supports_tray() {
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                HostServices::unavailable().with_tray(ui.tray.clone()),
            )?;
        (server, invitation, None)
    } else if config.supports_notification() {
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                HostServices::unavailable().with_notifications(ui.notifications.clone()),
            )?;
        (server, invitation, None)
    } else if config.supports_fields() {
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                HostServices::unavailable().with_ui_fields(ui.fields.clone()),
            )?;
        (server, invitation, None)
    } else if config.supports_window_controls() {
        let services = HostServices::unavailable()
            .with_window_title(ui.window_title.clone())
            .with_window_state(ui.window_state.clone())
            .with_window_state_read(ui.window_state_read.clone())
            .with_window_state_changes(ui.window_state_changes.clone())
            .with_window_focus(ui.window_focus.clone())
            .with_window_fullscreen(ui.window_fullscreen.clone())
            .with_window_size(ui.window_size.clone());
        let (server, invitation) =
            WindowsPipeServer::create_with_session_components_and_service_bundle(
                policy,
                config.session_id,
                ui.document.clone(),
                ui.input.clone(),
                ui.close.clone(),
                services,
            )?;
        (server, invitation, None)
    } else {
        let (server, invitation) = WindowsPipeServer::create_with_session_components(
            policy,
            config.session_id,
            ui.document.clone(),
            ui.input.clone(),
            ui.close.clone(),
        )?;
        (server, invitation, None)
    };
    let stop = server.stop_signal();
    let bootstrap = invitation.bootstrap_invitation()?;
    let worker = thread::spawn(move || server.serve_one());
    let child = match launch(&command, &bootstrap) {
        Ok(child) => child,
        Err(error) => {
            stop.request_stop();
            let _ = worker.join();
            return Err(error.into());
        }
    };

    let mut after_shown = Some(after_shown);
    let window_result = match window_group {
        Some(window_group) => {
            crate::win32::run_grouped_ui_session(window_group, ui.close, config.display_name)
        }
        None => {
            let close_for_observer = ui.close.clone();
            let after_shown = after_shown
                .take()
                .expect("non-grouped development session has one observer");
            crate::win32::run_ui_session_after_shown(
                ui.document,
                ui.input,
                ui.close,
                ui.file_dialog,
                ui.file_text,
                ui.folder_entries,
                ui.notifications,
                ui.menu,
                ui.context_menu,
                ui.tray,
                ui.window_title,
                ui.window_state,
                ui.window_state_read,
                ui.window_state_changes,
                ui.window_focus,
                ui.window_fullscreen,
                ui.window_size,
                config.display_name,
                ui.fields,
                move |window| after_shown(window, close_for_observer),
            )
        }
    };
    if let Err(error) = window_result {
        let _ = child.terminate(DEVELOPMENT_STOP_CODE);
        stop.request_stop();
        let _ = worker.join();
        return Err(error.into());
    }

    if let Err(error) = after_closed() {
        let _ = child.terminate(DEVELOPMENT_STOP_CODE);
        stop.request_stop();
        let _ = worker.join();
        return Err(error.into());
    }

    let exit_code = match child.wait_for_exit(CHILD_TIMEOUT_MILLISECONDS) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            let _ = child.terminate(DEVELOPMENT_STOP_CODE);
            stop.request_stop();
            let _ = worker.join();
            return Err(error.into());
        }
    };
    if exit_code != 0 {
        stop.request_stop();
        let _ = worker.join();
        return Err(io::Error::other(format!(
            "compiled native UI development session failed at safe stage {exit_code}"
        ))
        .into());
    }
    worker
        .join()
        .map_err(|_| io::Error::other("native UI development session pipe worker panicked"))??;
    println!("{}", config.completion_message);
    Ok(())
}
