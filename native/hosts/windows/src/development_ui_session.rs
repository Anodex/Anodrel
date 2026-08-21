//! Shared lifecycle for fixed-grant development native UI sessions.
//!
//! Each caller chooses only fixed host constants. The selected child supplies
//! no identity, title, capability, endpoint, or native window information.

use std::{error::Error, io, thread};

use anodrel_core::{HostPolicy, HostServices};
use anodrel_protocol::Capability;
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
const MENU_GRANTS: [Capability; 4] = [
    Capability::UiDocumentWrite,
    Capability::UiEventsRead,
    Capability::MenuWrite,
    Capability::SessionClose,
];

#[derive(Clone, Copy)]
enum DevelopmentUiSessionKind {
    Document,
    Menu,
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

    const fn grants(self) -> &'static [Capability] {
        match self.kind {
            DevelopmentUiSessionKind::Document => &UI_GRANTS,
            DevelopmentUiSessionKind::Menu => &MENU_GRANTS,
        }
    }

    const fn supports_menu(self) -> bool {
        matches!(self.kind, DevelopmentUiSessionKind::Menu)
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
    // Validate the selected command before a worker is allowed to wait for a
    // process that cannot start.
    let command = BootstrapCommand::new(client_path)?;
    let ui = DevelopmentSessionUi::new();
    let policy = HostPolicy::new(config.application_id, config.grants().to_vec(), HOST_NAME)?;
    let (server, invitation) = if config.supports_menu() {
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            config.session_id,
            ui.document.clone(),
            ui.input.clone(),
            ui.close.clone(),
            HostServices::unavailable().with_menu(ui.menu.clone()),
        )?
    } else {
        WindowsPipeServer::create_with_session_components(
            policy,
            config.session_id,
            ui.document.clone(),
            ui.input.clone(),
            ui.close.clone(),
        )?
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

    if let Err(error) = crate::win32::run_ui_session(
        ui.document,
        ui.input,
        ui.close,
        ui.file_dialog,
        ui.file_text,
        ui.notifications,
        ui.menu,
        ui.window_title,
        ui.window_state,
        ui.window_focus,
        config.display_name,
        ui.fields,
    ) {
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

    use super::{DevelopmentUiSessionConfig, MENU_GRANTS, UI_GRANTS};

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
        assert_eq!(document.application_id, "anodrel.test");
        assert_eq!(document.session_id, "test-session");
        assert_eq!(document.display_name, "Anodrel Test");
        assert_eq!(document.completion_message, "completed");
        assert_eq!(document.grants(), UI_GRANTS);
        assert!(!document.supports_menu());
        assert_eq!(menu.grants(), MENU_GRANTS);
        assert!(menu.supports_menu());
        assert_eq!(
            UI_GRANTS,
            [
                Capability::UiDocumentWrite,
                Capability::UiEventsRead,
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
    }
}
