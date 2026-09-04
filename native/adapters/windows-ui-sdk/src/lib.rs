#![forbid(unsafe_code)]

//! Stable Windows entry point for one invited Anodrel UI session.
//!
//! [`WindowsUiSession::connect_from_stdin`] consumes the one private bootstrap
//! invitation supplied by the host, opens only its invited named pipe, and
//! authenticates before exposing any typed UI operation. Applications cannot
//! construct a pipe endpoint, send a raw request, select a protocol version, or
//! choose a capability through this crate. See `docs/WINDOWS_NATIVE_SDK.md`.

use std::{fmt, io};

use anodrel_client::Client;
use anodrel_ui_client::UiSession;
use anodrel_windows_client::WindowsClientStream;

pub use anodrel_client::InteractivePollSchedule;
pub use anodrel_ui_client::{
    ContextMenuRevision, DocumentRevision, FileBinaryData, FileDialogFilter, MenuRevision,
    SaveReference, SaveSelection, SaveSelectionResult, SecondaryWindowId, SessionWindowId,
    TrayRevision, UiAction, UiActionBatch, UiClientError, UiContextMenuAction,
    UiContextMenuActionBatch, UiEvent, UiEventBatch, UiFieldSnapshot, UiFieldValue, UiTrayAction,
    UiTrayActionBatch, WindowFullscreenMode, WindowSize, WindowState, WindowUiAction,
    WindowUiActionBatch,
};

/// Closed outcomes while establishing one invited Windows UI session.
///
/// These values intentionally retain no bootstrap material, endpoint name,
/// native error, raw transport response, capability, or host diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowsUiConnectionError {
    /// Standard input did not contain exactly one valid host invitation.
    BootstrapUnavailable,
    /// The exact invited Windows pipe could not be opened.
    InvitedEndpointUnavailable,
    /// The invited pipe could not complete the required authentication exchange.
    AuthenticationUnavailable,
}

impl fmt::Display for WindowsUiConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::BootstrapUnavailable => "the private bootstrap invitation was unavailable",
            Self::InvitedEndpointUnavailable => "the invited Windows endpoint was unavailable",
            Self::AuthenticationUnavailable => "the invited session could not authenticate",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowsUiConnectionError {}

/// One authenticated Windows-native UI session with a closed typed surface.
///
/// The session owns its one invited stream. It exposes only the documented
/// typed UI calls and never returns the underlying stream, client, endpoint,
/// invitation, or native handle.
pub struct WindowsUiSession {
    session: UiSession<WindowsClientStream>,
}

impl WindowsUiSession {
    /// Consumes the single private invitation supplied on standard input and
    /// authenticates an exact invited Windows named-pipe session.
    ///
    /// No argument can select a pipe, credential, protocol version, host, or
    /// capability. The host remains authoritative for all subsequent requests.
    pub fn connect_from_stdin() -> Result<Self, WindowsUiConnectionError> {
        let mut input = io::stdin().lock();
        let session = establish_session(&mut input, WindowsClientStream::connect)?;
        Ok(Self { session })
    }

    /// Replaces this session's primary surface with one strict version-1 document.
    pub fn replace_document_v1(
        &mut self,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        self.session.replace_document_v1(document)
    }

    /// Replaces this session's primary surface with one strict version-3 document.
    pub fn replace_document_v3(
        &mut self,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        self.session.replace_document_v3(document)
    }

    /// Drains one bounded batch of revision-bound document action events.
    pub fn read_actions(&mut self) -> Result<UiActionBatch, UiClientError> {
        self.session.read_actions()
    }

    /// Reads one explicit whole-surface field snapshot for this session.
    pub fn read_fields(&mut self) -> Result<UiFieldSnapshot, UiClientError> {
        self.session.read_fields()
    }

    /// Replaces this session's complete strict native menu model.
    pub fn replace_menu_v1(&mut self, menu: &str) -> Result<MenuRevision, UiClientError> {
        self.session.replace_menu_v1(menu)
    }

    /// Replaces this session's complete host-owned native context-menu model.
    pub fn replace_context_menu_v1(
        &mut self,
        menu: &str,
    ) -> Result<ContextMenuRevision, UiClientError> {
        self.session.replace_context_menu_v1(menu)
    }

    /// Drains one bounded batch of document and native-menu semantic events.
    pub fn read_events(&mut self) -> Result<UiEventBatch, UiClientError> {
        self.session.read_events()
    }

    /// Drains one batch containing only local context-menu semantic actions.
    pub fn read_context_menu_actions(&mut self) -> Result<UiContextMenuActionBatch, UiClientError> {
        self.session.read_context_menu_actions()
    }

    /// Replaces this session's complete host-owned notification-area tray model.
    pub fn replace_tray_v1(&mut self, menu: &str) -> Result<TrayRevision, UiClientError> {
        self.session.replace_tray_v1(menu)
    }

    /// Drains one batch containing only notification-area tray actions.
    pub fn read_tray_actions(&mut self) -> Result<UiTrayActionBatch, UiClientError> {
        self.session.read_tray_actions()
    }

    /// Asks the host to hand one bounded notification to its native surface.
    ///
    /// Acceptance does not reveal whether a person saw, dismissed, or acted on
    /// the notification. The host alone owns its Shell32 entry and artwork.
    pub fn show_notification(&mut self, title: &str, body: &str) -> Result<(), UiClientError> {
        self.session.show_notification(title, body)
    }

    /// Opens one host-owned save picker and captures one retained output object.
    ///
    /// The selected path is display data only. Writing requires the opaque,
    /// one-use reference from the selected result.
    pub fn select_save_file_v2(
        &mut self,
        filters: &[FileDialogFilter],
    ) -> Result<SaveSelectionResult, UiClientError> {
        self.session.select_save_file_v2(filters)
    }

    /// Writes one bounded UTF-8 value through a retained selected output.
    ///
    /// This does not accept a path, native handle, offset, append flag, or a
    /// durable/atomicity option.
    pub fn write_selected_text(
        &mut self,
        reference: &SaveReference,
        text: &str,
    ) -> Result<(), UiClientError> {
        self.session.write_selected_text(reference, text)
    }

    /// Writes one bounded binary value through a retained selected output.
    ///
    /// The typed data value carries one canonical base64url spelling. This
    /// method does not accept a path, native handle, MIME type, offset, append
    /// flag, stream, or a durable/atomicity option.
    pub fn write_selected_binary(
        &mut self,
        reference: &SaveReference,
        data: &FileBinaryData,
    ) -> Result<(), UiClientError> {
        self.session.write_selected_binary(reference, data)
    }

    /// Proposes a title for this session's own host window.
    pub fn set_window_title(&mut self, title: &str) -> Result<(), UiClientError> {
        self.session.set_window_title(title)
    }

    /// Requests one closed presentation state for this session's own host window.
    pub fn set_window_state(&mut self, state: WindowState) -> Result<(), UiClientError> {
        self.session.set_window_state(state)
    }

    /// Asks Windows to foreground this session's own host window.
    pub fn request_window_focus(&mut self) -> Result<(), UiClientError> {
        self.session.request_window_focus()
    }

    /// Requests one reversible presentation mode for this session's own host window.
    pub fn set_window_fullscreen(
        &mut self,
        mode: WindowFullscreenMode,
    ) -> Result<(), UiClientError> {
        self.session.set_window_fullscreen(mode)
    }

    /// Requests one bounded logical client size for this session's own host window.
    pub fn set_window_size(&mut self, size: WindowSize) -> Result<(), UiClientError> {
        self.session.set_window_size(size)
    }

    /// Opens one session-owned secondary view with a strict version-1 document.
    pub fn open_window_v1(
        &mut self,
        title: &str,
        document: &str,
    ) -> Result<SecondaryWindowId, UiClientError> {
        self.session.open_window_v1(title, document)
    }

    /// Opens one session-owned secondary view with a strict version-2 document.
    pub fn open_window_v2(
        &mut self,
        title: &str,
        document: &str,
    ) -> Result<SecondaryWindowId, UiClientError> {
        self.session.open_window_v2(title, document)
    }

    /// Opens one session-owned secondary view with a strict version-3 document.
    pub fn open_window_v3(
        &mut self,
        title: &str,
        document: &str,
    ) -> Result<SecondaryWindowId, UiClientError> {
        self.session.open_window_v3(title, document)
    }

    /// Replaces one opened secondary view with a strict version-1 document.
    pub fn replace_window_document_v1(
        &mut self,
        window: SecondaryWindowId,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        self.session.replace_window_document_v1(window, document)
    }

    /// Replaces one opened secondary view with a strict version-2 document.
    pub fn replace_window_document_v2(
        &mut self,
        window: SecondaryWindowId,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        self.session.replace_window_document_v2(window, document)
    }

    /// Replaces one opened secondary view with a strict version-3 document.
    pub fn replace_window_document_v3(
        &mut self,
        window: SecondaryWindowId,
        document: &str,
    ) -> Result<DocumentRevision, UiClientError> {
        self.session.replace_window_document_v3(window, document)
    }

    /// Drains bounded revision-checked document actions from every session view.
    pub fn read_window_actions(&mut self) -> Result<WindowUiActionBatch, UiClientError> {
        self.session.read_window_actions()
    }

    /// Requests close of one opaque secondary view returned by this session.
    pub fn close_window(&mut self, window: SecondaryWindowId) -> Result<(), UiClientError> {
        self.session.close_window(window)
    }

    /// Requests host-owned closure of this authenticated session group.
    pub fn close(&mut self) -> Result<(), UiClientError> {
        self.session.close()
    }
}

impl fmt::Debug for WindowsUiSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsUiSession")
            .finish_non_exhaustive()
    }
}

fn establish_session<Stream>(
    input: &mut impl io::Read,
    open: impl FnOnce(&anodrel_bootstrap::BootstrapInvitation) -> io::Result<Stream>,
) -> Result<UiSession<Stream>, WindowsUiConnectionError>
where
    Stream: io::Read + io::Write,
{
    let invitation = Client::<Stream>::read_invitation(input)
        .map_err(|_| WindowsUiConnectionError::BootstrapUnavailable)?;
    let stream =
        open(&invitation).map_err(|_| WindowsUiConnectionError::InvitedEndpointUnavailable)?;
    let client = Client::authenticate(stream, invitation)
        .map_err(|_| WindowsUiConnectionError::AuthenticationUnavailable)?;
    Ok(UiSession::new(client))
}

#[cfg(test)]
mod tests;
