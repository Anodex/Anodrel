//! Shared lifecycle for fixed-grant development native UI sessions.
//!
//! Each caller chooses only fixed host constants. The selected child supplies
//! no identity, capability, endpoint, or native window information. A route
//! may accept one bounded typed proposal only when its fixed grant permits it.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_protocol::Capability;
use anodrel_ui_session::UiWindowGroup;
use anodrel_window::WindowTitleProposal;
use anodrel_windows_bootstrap::{BootstrapCommand, launch};
use anodrel_windows_pipe::WindowsPipeServer;

use crate::session_ui::DevelopmentSessionUi;

const CHILD_TIMEOUT_MILLISECONDS: u32 = 10_000;
const DEVELOPMENT_STOP_CODE: u32 = 0xA11D;
const HOST_NAME: &str = "anodrel-windows-host";
const UI_GRANTS: [Capability; 3] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::SessionClose,
];
const FORM_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::UiFieldsRead,
    Capability::SessionClose,
];
const MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::MenuWrite,
    Capability::SessionClose,
];
const MULTI_WINDOW_GRANTS: [Capability; 5] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowOpen,
    Capability::WindowClose,
    Capability::SessionClose,
];
const WINDOW_CONTROLS_GRANTS: [Capability; 8] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::WindowTitle,
    Capability::WindowState,
    Capability::WindowFocus,
    Capability::WindowFullscreen,
    Capability::WindowSize,
    Capability::SessionClose,
];

#[derive(Clone, Copy)]
enum DevelopmentUiSessionKind {
    Document,
    Form,
    Menu,
    MultiWindow,
    WindowControls,
}

/// Fixed host facts for one explicitly selected development child route.
#[derive(Clone, Copy)]
pub(crate) struct DevelopmentUiSessionConfig {
    application_id: &'static str,
    session_id: &'static str,
    display_name: &'static str,
    completion_message: &'static str,
    kind: DevelopmentUiSessionKind,
}

impl DevelopmentUiSessionConfig {
    /// Creates a configuration whose only session permissions are UI write,
    /// semantic-action read, and self-close.
    pub(crate) const fn new(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Document,
        }
    }

    /// Creates a configuration for the explicit whole-surface field-read
    /// route. The other templates do not acquire this inward-facing authority.
    pub(crate) const fn with_form(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Form,
        }
    }

    /// Creates a configuration whose only additional session permission is a
    /// complete bounded native-menu replacement.
    pub(crate) const fn with_menu(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::Menu,
        }
    }

    /// Creates a configuration for the explicit bounded multi-window route.
    ///
    /// Window creation and secondary close are additional fixed grants on this
    /// distinct route. The normal and menu routes remain narrower.
    pub(crate) const fn with_multi_window(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::MultiWindow,
        }
    }

    /// Creates a configuration for the explicit targetless session-window
    /// controls route. No existing development template acquires these grants.
    pub(crate) const fn with_window_controls(
        application_id: &'static str,
        session_id: &'static str,
        display_name: &'static str,
        completion_message: &'static str,
    ) -> Self {
        Self {
            application_id,
            session_id,
            display_name,
            completion_message,
            kind: DevelopmentUiSessionKind::WindowControls,
        }
    }

    const fn grants(self) -> &'static [Capability] {
        match self.kind {
            DevelopmentUiSessionKind::Document => &UI_GRANTS,
            DevelopmentUiSessionKind::Form => &FORM_GRANTS,
            DevelopmentUiSessionKind::Menu => &MENU_GRANTS,
            DevelopmentUiSessionKind::MultiWindow => &MULTI_WINDOW_GRANTS,
            DevelopmentUiSessionKind::WindowControls => &WINDOW_CONTROLS_GRANTS,
        }
    }

    const fn supports_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Menu)
    }

    const fn supports_fields(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Form)
    }

    const fn supports_multi_window(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::MultiWindow)
    }

    const fn supports_window_controls(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::WindowControls)
    }
}

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
                ui.window_title,
                ui.window_state,
                ui.window_state_read,
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

#[cfg(test)]
mod tests {
    use anodrel_protocol::Capability;

    use super::{
        DevelopmentUiSessionConfig, FORM_GRANTS, MENU_GRANTS, MULTI_WINDOW_GRANTS, UI_GRANTS,
        WINDOW_CONTROLS_GRANTS,
    };

    #[test]
    fn development_routes_use_only_their_exact_closed_grant_sets() {
        let document = DevelopmentUiSessionConfig::new(
            "anodrel.test",
            "test-session",
            "Anodrel Test",
            "completed",
        );
        let menu = DevelopmentUiSessionConfig::with_menu(
            "anodrel.test-menu",
            "test-menu-session",
            "Anodrel Menu Test",
            "completed menu",
        );
        let form = DevelopmentUiSessionConfig::with_form(
            "anodrel.test-form",
            "test-form-session",
            "Anodrel Form Test",
            "completed form",
        );
        let multi_window = DevelopmentUiSessionConfig::with_multi_window(
            "anodrel.test-multi-window",
            "test-multi-window-session",
            "Anodrel Multi-Window Test",
            "completed multi-window",
        );
        let window_controls = DevelopmentUiSessionConfig::with_window_controls(
            "anodrel.test-window-controls",
            "test-window-controls-session",
            "Anodrel Window Controls Test",
            "completed window controls",
        );
        assert_eq!(document.application_id, "anodrel.test");
        assert_eq!(document.session_id, "test-session");
        assert_eq!(document.display_name, "Anodrel Test");
        assert_eq!(document.completion_message, "completed");
        assert_eq!(document.grants(), UI_GRANTS);
        assert!(!document.supports_menu());
        assert!(!document.supports_fields());
        assert_eq!(form.grants(), FORM_GRANTS);
        assert!(form.supports_fields());
        assert!(!form.supports_menu());
        assert_eq!(menu.grants(), MENU_GRANTS);
        assert!(menu.supports_menu());
        assert_eq!(multi_window.grants(), MULTI_WINDOW_GRANTS);
        assert!(!multi_window.supports_menu());
        assert!(multi_window.supports_multi_window());
        assert_eq!(window_controls.grants(), WINDOW_CONTROLS_GRANTS);
        assert!(window_controls.supports_window_controls());
        assert!(!window_controls.supports_menu());
        assert!(!window_controls.supports_fields());
        assert!(!window_controls.supports_multi_window());
        assert_eq!(
            UI_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            FORM_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::UiFieldsRead,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            MENU_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::MenuWrite,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            MULTI_WINDOW_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::WindowOpen,
                Capability::WindowClose,
                Capability::SessionClose,
            ]
        );
        assert_eq!(
            WINDOW_CONTROLS_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
                Capability::WindowTitle,
                Capability::WindowState,
                Capability::WindowFocus,
                Capability::WindowFullscreen,
                Capability::WindowSize,
                Capability::SessionClose,
            ]
        );
    }
}
