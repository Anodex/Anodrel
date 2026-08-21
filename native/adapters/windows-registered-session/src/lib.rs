#![forbid(unsafe_code)]

//! Windows registered-application session setup.
//!
//! This adapter joins a validated, machine-selected application policy to one
//! owner-restricted Windows named-pipe session. Its optional interactive path
//! also creates the one grouped set of host-owned resources consumed by an
//! authenticated native window. It does not launch a process, deliver a
//! bootstrap invitation, select an application ID, or serve pipe I/O; callers
//! retain those lifecycle responsibilities.

use std::{fmt, io};

use anodrel_application::InstalledApplication;
use anodrel_core::{HostPolicy, HostServices, SessionCloseSignal};
use anodrel_file_access::{SaveFileDialogMailbox, SelectionFileDialogMailbox};
use anodrel_file_dialog::FileDialogMailbox;
use anodrel_menu::MenuMailbox;
use anodrel_notifications::NotificationMailbox;
use anodrel_session_policy::host_policy_for_installed_application;
use anodrel_ui_session::{UiDocumentMailbox, UiFieldMailbox, UiInputMailbox};
use anodrel_window::{WindowFocusMailbox, WindowStateMailbox, WindowTitleMailbox};
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_paths::{WindowsPathsError, application_directories};
use anodrel_windows_pipe::{SessionInvitation, WindowsPipeServer};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_storage::WindowsStorageService;

/// The host-owned native UI resources for one registered application session.
///
/// This group has no native handle, process, title, launch operation, or
/// application-selected data. Host code must pass it only to the native window
/// that belongs to the same session returned by [`RegisteredUiSession`].
#[derive(Clone, Debug)]
pub struct RegisteredSessionUi {
    document_mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    notification_mailbox: NotificationMailbox,
    menu_mailbox: MenuMailbox,
    window_title_mailbox: WindowTitleMailbox,
    window_state_mailbox: WindowStateMailbox,
    window_focus_mailbox: WindowFocusMailbox,
    field_mailbox: UiFieldMailbox,
    /// The display name the host appends to any title this session proposes.
    ///
    /// Taken from the identity that matched the machine-validated installed
    /// record, so it is not something the application can influence at run
    /// time. Carrying it here is what lets the UI thread compose a caption an
    /// application cannot forge. See `docs/WINDOW_TITLE.md`.
    display_name: String,
}

impl RegisteredSessionUi {
    fn new(display_name: impl Into<String>) -> Self {
        Self {
            document_mailbox: UiDocumentMailbox::new(),
            input_mailbox: UiInputMailbox::new(),
            close_signal: SessionCloseSignal::default(),
            file_dialog_mailbox: FileDialogMailbox::new(),
            file_text: WindowsFileTextService::new(),
            notification_mailbox: NotificationMailbox::new(),
            menu_mailbox: MenuMailbox::new(),
            window_title_mailbox: WindowTitleMailbox::new(),
            window_state_mailbox: WindowStateMailbox::new(),
            window_focus_mailbox: WindowFocusMailbox::new(),
            field_mailbox: UiFieldMailbox::new(),
            display_name: display_name.into(),
        }
    }

    /// Returns this session's document-delivery mailbox.
    #[must_use]
    pub fn document_mailbox(&self) -> UiDocumentMailbox {
        self.document_mailbox.clone()
    }

    /// Returns this session's bounded semantic-input mailbox.
    #[must_use]
    pub fn input_mailbox(&self) -> UiInputMailbox {
        self.input_mailbox.clone()
    }

    /// Returns this session's host-owned close signal.
    #[must_use]
    pub fn close_signal(&self) -> SessionCloseSignal {
        self.close_signal.clone()
    }

    /// Returns this session's one-request UI-thread dialog mailbox.
    #[must_use]
    pub fn file_dialog_mailbox(&self) -> FileDialogMailbox {
        self.file_dialog_mailbox.clone()
    }

    /// Returns this session's retained selected-file text service.
    #[must_use]
    pub fn file_text_service(&self) -> WindowsFileTextService {
        self.file_text.clone()
    }

    /// Returns this session's one-request UI-thread notification mailbox.
    #[must_use]
    pub fn notification_mailbox(&self) -> NotificationMailbox {
        self.notification_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread menu mailbox.
    ///
    /// It carries only a complete validated semantic model and host-owned
    /// revision. The UI thread that owns this session's window supplies every
    /// native menu object and private command identifier.
    #[must_use]
    pub fn menu_mailbox(&self) -> MenuMailbox {
        self.menu_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-title mailbox.
    #[must_use]
    pub fn window_title_mailbox(&self) -> WindowTitleMailbox {
        self.window_title_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-state mailbox.
    ///
    /// It transfers only the closed minimise, maximise, or restore value. The
    /// native window remains resolved by the host, never by the application.
    #[must_use]
    pub fn window_state_mailbox(&self) -> WindowStateMailbox {
        self.window_state_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread window-focus mailbox.
    ///
    /// It carries no target, native handle, retry policy, or focus readback.
    /// The owning UI thread resolves the one session window; see
    /// `docs/WINDOW_FOCUS.md`.
    #[must_use]
    pub fn window_focus_mailbox(&self) -> WindowFocusMailbox {
        self.window_focus_mailbox.clone()
    }

    /// Returns this session's one-request UI-thread field-read mailbox.
    #[must_use]
    pub fn field_mailbox(&self) -> UiFieldMailbox {
        self.field_mailbox.clone()
    }

    /// Returns the validated display name the host appends to a proposed title.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

/// One registered interactive session before a host starts pipe I/O.
///
/// The pipe endpoint, sensitive invitation, and UI group are created together
/// from the same validated machine policy record. Host code must keep them in
/// the same launch and shutdown lifecycle.
pub struct RegisteredUiSession {
    server: WindowsPipeServer,
    invitation: SessionInvitation,
    ui: RegisteredSessionUi,
}

impl RegisteredUiSession {
    /// Separates the endpoint and invitation from the grouped native UI
    /// resources for explicit host lifecycle ownership.
    #[must_use]
    pub fn into_parts(self) -> (WindowsPipeServer, SessionInvitation, RegisteredSessionUi) {
        (self.server, self.invitation, self.ui)
    }
}

impl fmt::Debug for RegisteredUiSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredUiSession(..)")
    }
}

/// Creates one authenticated named-pipe session for a host-selected registered
/// application.
///
/// `application_id`, `host_name`, and `session_id` must be supplied by native
/// host policy, never by package, bootstrap, transport, protocol, or UI data.
/// The sensitive invitation remains separate from the endpoint and must be
/// delivered through the existing private bootstrap channel only after the
/// caller has completed executable trust checks.
pub fn create_registered_session(
    application_id: &str,
    host_name: impl Into<String>,
    session_id: impl Into<String>,
) -> Result<(WindowsPipeServer, SessionInvitation), RegisteredSessionError> {
    let application =
        load_installed_application(application_id).map_err(RegisteredSessionError::Policy)?;
    let policy = host_policy_for_installed_application(&application, host_name)
        .map_err(|_| RegisteredSessionError::InvalidHostName)?;
    let services = registered_services(&application)?;
    create_session_with_services(policy, session_id, services).map_err(RegisteredSessionError::Pipe)
}

/// Creates one interactive session for a host-selected registered application.
///
/// The returned resources are attached to the authenticated transport before
/// the pipe peer connects. This does not start a process, create a window, or
/// deliver the invitation; a verified host lifecycle owns those steps.
pub fn create_registered_ui_session(
    application_id: &str,
    host_name: impl Into<String>,
    session_id: impl Into<String>,
) -> Result<RegisteredUiSession, RegisteredSessionError> {
    let application =
        load_installed_application(application_id).map_err(RegisteredSessionError::Policy)?;
    let policy = host_policy_for_installed_application(&application, host_name)
        .map_err(|_| RegisteredSessionError::InvalidHostName)?;
    let ui = RegisteredSessionUi::new(application.identity().display_name());
    let services = registered_interactive_services(&application, &ui)?;
    let (server, invitation) =
        WindowsPipeServer::create_with_session_components_and_service_bundle(
            policy,
            session_id,
            ui.document_mailbox(),
            ui.input_mailbox(),
            ui.close_signal(),
            services,
        )
        .map_err(RegisteredSessionError::Pipe)?;
    Ok(RegisteredUiSession {
        server,
        invitation,
        ui,
    })
}

#[cfg(test)]
fn create_session(
    policy: HostPolicy,
    session_id: impl Into<String>,
) -> io::Result<(WindowsPipeServer, SessionInvitation)> {
    WindowsPipeServer::create(policy, session_id)
}

fn create_session_with_services(
    policy: HostPolicy,
    session_id: impl Into<String>,
    services: HostServices,
) -> io::Result<(WindowsPipeServer, SessionInvitation)> {
    WindowsPipeServer::create_with_services(policy, session_id, services)
}

fn registered_services(
    application: &InstalledApplication,
) -> Result<HostServices, RegisteredSessionError> {
    let directories = application_directories(application.identity())
        .map_err(RegisteredSessionError::Directories)?;
    Ok(HostServices::unavailable()
        .with_clipboard(WindowsClipboard::new(0))
        .with_external_links(WindowsExternalLinks)
        .with_storage(WindowsStorageService::new(&directories))
        .with_credentials(WindowsCredentialService::new(
            application.identity().clone(),
        )))
}

fn registered_interactive_services(
    application: &InstalledApplication,
    ui: &RegisteredSessionUi,
) -> Result<HostServices, RegisteredSessionError> {
    Ok(registered_services(application)?
        .with_file_dialogs(ui.file_dialog_mailbox())
        .with_file_selections(SelectionFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_file_text(ui.file_text_service())
        .with_file_save_selections(SaveFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_file_text_write(ui.file_text_service().write_service())
        // Notifications reach Shell32 through the owning UI thread, so the
        // session gets the mailbox rather than the adapter.
        .with_notifications(ui.notification_mailbox())
        // A complete semantic menu reaches User32 only through this session's
        // owning UI thread; no pipe worker gains a native menu handle.
        .with_menu(ui.menu_mailbox())
        // A window caption reaches User32 the same way, and the UI thread holds
        // the validated display name it composes with.
        .with_window_title(ui.window_title_mailbox())
        // A presentation state takes the same host-only UI-thread path and is
        // still resolved from this session rather than a caller-supplied target.
        .with_window_state(ui.window_state_mailbox())
        // Foregrounding stays in the same session-owned UI-thread boundary.
        // The policy parser admits this mailbox only for record version 1.9.
        .with_window_focus(ui.window_focus_mailbox())
        // Field values live with the window that owns them, so a read crosses
        // to the UI thread the same way. See `docs/UI_FIELDS.md`.
        .with_ui_fields(ui.field_mailbox()))
}

/// A safe failure category while creating a registered application session.
#[derive(Debug)]
pub enum RegisteredSessionError {
    Policy(PolicyStoreError),
    InvalidHostName,
    Directories(WindowsPathsError),
    Pipe(io::Error),
}

impl fmt::Display for RegisteredSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(_) => {
                formatter.write_str("registered application policy could not be loaded")
            }
            Self::InvalidHostName => {
                formatter.write_str("registered application session host name is invalid")
            }
            Self::Directories(_) => formatter
                .write_str("registered application service directories could not be derived"),
            Self::Pipe(_) => {
                formatter.write_str("registered application session could not be created")
            }
        }
    }
}

impl std::error::Error for RegisteredSessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::Directories(error) => Some(error),
            Self::Pipe(error) => Some(error),
            Self::InvalidHostName => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_core::HostPolicy;
    use anodrel_protocol::Capability;
    use anodrel_windows_policy::PolicyStoreError;

    use super::{
        RegisteredSessionError, RegisteredSessionUi, create_registered_session, create_session,
    };

    #[test]
    fn rejects_an_invalid_application_id_before_creating_a_pipe() {
        assert!(matches!(
            create_registered_session("org.anodrel/escape", "windows-host", "test-session"),
            Err(RegisteredSessionError::Policy(
                PolicyStoreError::InvalidApplicationId
            ))
        ));
    }

    #[test]
    fn creates_an_owner_restricted_session_from_a_host_policy() {
        let policy = HostPolicy::new(
            "org.anodrel.sample",
            vec![Capability::DiagnosticsRead],
            "windows-host",
        )
        .expect("fixture host policy is valid");

        let (_server, invitation) =
            create_session(policy, "test-session").expect("session is created");
        assert!(invitation.pipe_name().starts_with(r"\\.\pipe\anodrel.v1."));
        assert_eq!(invitation.session_id(), "test-session");
    }

    #[test]
    fn interactive_ui_resources_keep_their_close_signal_session_local() {
        let first = RegisteredSessionUi::new("First Application");
        let second = RegisteredSessionUi::new("Second Application");

        first.close_signal().request();
        assert!(first.close_signal().take());
        assert!(!second.close_signal().take());
    }

    #[test]
    fn each_session_carries_its_own_title_bridge_and_validated_name() {
        // Two sessions must never share either half. A shared mailbox would let
        // one application's proposal be applied to another's window, and a
        // shared name would let a caption claim the wrong application.
        let first = RegisteredSessionUi::new("First Application");
        let second = RegisteredSessionUi::new("Second Application");

        assert_eq!(first.display_name(), "First Application");
        assert_eq!(second.display_name(), "Second Application");

        let proposal =
            anodrel_window::WindowTitleProposal::new("Report").expect("the proposal is valid");
        let bridge = first.window_title_mailbox();
        let waiting = std::thread::spawn(move || {
            anodrel_window::WindowTitleService::set_title(&bridge, &proposal)
        });
        while first.window_title_mailbox().take().is_none() {
            std::thread::yield_now();
        }
        // The pending proposal belongs to the session that made it, and the
        // other session's bridge knows nothing about it.
        assert!(second.window_title_mailbox().take().is_none());
        assert!(first.window_title_mailbox().fail(1));
        assert!(waiting.join().expect("the worker did not panic").is_err());
    }

    #[test]
    fn each_session_carries_its_own_closed_window_state_bridge() {
        let first = RegisteredSessionUi::new("First Application");
        let second = RegisteredSessionUi::new("Second Application");
        let bridge = first.window_state_mailbox();
        let waiting = std::thread::spawn(move || {
            anodrel_window::WindowStateService::set_state(
                &bridge,
                anodrel_window::WindowState::Minimized,
            )
        });

        while first.window_state_mailbox().take().is_none() {
            assert!(
                second.window_state_mailbox().take().is_none(),
                "a session took another session's presentation command"
            );
            std::thread::yield_now();
        }
        assert!(second.window_state_mailbox().take().is_none());
        assert!(first.window_state_mailbox().fail(1));
        assert!(waiting.join().expect("the worker did not panic").is_err());
    }

    #[test]
    fn each_session_carries_its_own_window_focus_bridge() {
        let first = RegisteredSessionUi::new("First Application");
        let second = RegisteredSessionUi::new("Second Application");
        let bridge = first.window_focus_mailbox();
        let waiting =
            std::thread::spawn(move || anodrel_window::WindowFocusService::request_focus(&bridge));

        while first.window_focus_mailbox().take().is_none() {
            assert!(
                second.window_focus_mailbox().take().is_none(),
                "a session took another session's focus request"
            );
            std::thread::yield_now();
        }
        assert!(second.window_focus_mailbox().take().is_none());
        assert!(first.window_focus_mailbox().fail(1));
        assert!(waiting.join().expect("the worker did not panic").is_err());
    }
}
