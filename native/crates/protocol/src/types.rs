//! Stable protocol version, capability, and failure-code values.

use super::{JsonValue, PROTOCOL_MAJOR, PROTOCOL_MINOR, object};

/// The declared protocol version on one message envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolVersion {
    /// Protocol major version.
    pub major: u16,
    /// Protocol minor version.
    pub minor: u16,
}

impl ProtocolVersion {
    /// The newest protocol version this crate supports.
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };

    /// Returns whether a message version is compatible with this host.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.major == PROTOCOL_MAJOR && self.minor <= PROTOCOL_MINOR
    }

    /// Encodes the version in its public envelope shape.
    #[must_use]
    pub fn to_json(self) -> JsonValue {
        object([
            ("major", JsonValue::Number(self.major.to_string())),
            ("minor", JsonValue::Number(self.minor.to_string())),
        ])
    }
}

/// One explicitly granted host capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Read the bounded host diagnostic catalogue.
    DiagnosticsRead,
    /// Replace the current host-owned semantic UI document.
    UiDocumentWrite,
    /// Read current bounded semantic UI events.
    UiEventsRead,
    /// End the authenticated session.
    SessionClose,
    /// Read bounded Unicode text from the system clipboard.
    ClipboardRead,
    /// Write bounded Unicode text to the system clipboard.
    ClipboardWrite,
    /// Hand one validated HTTPS URI to the operating system.
    ExternalOpen,
    /// Fetch bounded text from one host-authorized HTTPS origin.
    ///
    /// The separate host service owns exact origin policy, direct network
    /// calls, and all native state. This capability accepts no method, body,
    /// header, cookie, credential, proxy, or connection handle.
    NetworkFetch,
    /// Open a host-owned picker for one file.
    DialogOpenFile,
    /// Show a host-owned picker for exactly one filesystem folder.
    ///
    /// The result is a display path only. It creates no retained folder
    /// permission, enumeration route, handle, or filesystem access surface.
    DialogOpenFolder,
    /// Read one bounded direct-entry snapshot through a selected-folder reference.
    ///
    /// This grant cannot select a folder, name a path, recurse, retrieve child
    /// contents or metadata, mutate the filesystem, or retain access after its
    /// one-use reference is consumed. See `docs/FOLDER_ACCESS.md`.
    FolderReadEntries,
    /// Open a host-owned save picker for one file.
    DialogSaveFile,
    /// Read bounded text through one selected file reference.
    FileReadText,
    /// Write bounded text through one selected output reference.
    FileWriteText,
    /// Write bounded decoded binary data through one retained output object.
    ///
    /// The grant has no path, file handle, stream, offset, or raw protocol
    /// byte surface; see `docs/FILE_BINARY_WRITE.md` and Decision 0087.
    FileWriteBinary,
    /// Read one bounded application state snapshot.
    StorageStateRead,
    /// Replace one bounded application state snapshot.
    StorageStateReplace,
    /// Clear one bounded application state snapshot.
    StorageStateClear,
    /// Read one exact credential.
    CredentialRead,
    /// Write one exact credential.
    CredentialWrite,
    /// Delete one exact credential.
    CredentialDelete,
    /// Show one bounded notification. There is no read counterpart to grant,
    /// because a notification has no read surface at all.
    NotificationShow,
    /// Propose the title of the session's own window.
    ///
    /// A proposal, not an assignment: the host composes the displayed caption
    /// with a validated application-name suffix. There is no read counterpart
    /// and no way to name a window. See `docs/WINDOW_TITLE.md`.
    WindowTitle,
    /// Request one standard presentation state for the session's own window.
    ///
    /// The state is a closed minimise/maximise/restore value. There is no
    /// target, event, or native command surface. See `docs/WINDOW_STATE.md`
    /// and Decision 0072.
    WindowState,
    /// Read one current standard state of the session's own window.
    ///
    /// This separate grant allows only one immediate minimized/maximized/
    /// restored snapshot. It exposes no target, handle, geometry, monitor,
    /// focus, fullscreen state, timestamp, or change event. See
    /// `docs/WINDOW_STATE_OBSERVATION.md` and Decision 0117.
    WindowStateRead,
    /// Read one coalesced presentation-state change for the session's own
    /// window.
    ///
    /// The result is only a latest minimized, maximized, or restored value,
    /// or no retained change. It exposes no target, handle, timestamp,
    /// sequence, history, wait, callback, or subscription. See
    /// `docs/WINDOW_STATE_CHANGES.md` and Decision 0118.
    WindowStateObserve,
    /// Ask Windows to foreground the session's own host window.
    ///
    /// The request has no target or focus-state readback. Windows may refuse
    /// it under its foreground rules; see `docs/WINDOW_FOCUS.md`.
    WindowFocus,
    /// Choose reversible borderless fullscreen for the session's own window.
    ///
    /// The host retains every native style and placement fact. This grant
    /// cannot select a monitor, change a display mode, set geometry, or read
    /// window state; see `docs/WINDOW_FULLSCREEN.md`.
    WindowFullscreen,
    /// Resize the client area of the session's own native window.
    ///
    /// The grant carries only bounded logical client dimensions. It cannot
    /// target or move a window, select a monitor, read geometry, or expose a
    /// native rectangle; see `docs/WINDOW_SIZE.md` and Decision 0088.
    WindowSize,
    /// Create one bounded secondary view in the authenticated session group.
    ///
    /// The application supplies only a validated caption proposal and strict
    /// UI document. The host alone creates and maps the native window; the
    /// returned identity is session-local and never a native handle.
    WindowOpen,
    /// Ask the host to close one known secondary session view.
    ///
    /// The primary remains the session anchor and can end only through the
    /// separately granted `session.close` operation. This grant exposes no
    /// close state, reason, event, or native target.
    WindowClose,
    /// Read every field value on the session's own current surface.
    ///
    /// A snapshot, not a stream. There is no selector and no change event, so
    /// this grant cannot be used to reconstruct what someone is typing. See
    /// `docs/UI_FIELDS.md` and Decision 0067.
    UiFieldsRead,
    /// Replace the complete native menu for this authenticated session.
    ///
    /// The application supplies semantic labels and enabled command IDs only.
    /// A host owns native identifiers, window attachment, and activation
    /// routing; see `docs/MENUS.md`.
    MenuWrite,
    /// Replace the complete host-owned context menu for this session.
    ///
    /// The application supplies only bounded semantic items. The host alone
    /// owns local trigger handling, placement, native popup construction, and
    /// command routing; see `docs/CONTEXT_MENUS.md`.
    ContextMenuWrite,
    /// Replace the complete host-owned tray menu for this session.
    ///
    /// The application supplies only a bounded semantic action model. The host
    /// retains the notification-area icon, popup placement, native command
    /// identifiers, and every local click route; see `docs/TRAY.md`.
    TrayWrite,
}

impl Capability {
    /// Returns the stable protocol spelling of this capability.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DiagnosticsRead => "diagnostics.read",
            Self::UiDocumentWrite => "ui.document.write",
            Self::UiEventsRead => "ui.events.read",
            Self::SessionClose => "session.close",
            Self::ClipboardRead => "clipboard.read",
            Self::ClipboardWrite => "clipboard.write",
            Self::ExternalOpen => "external.open",
            Self::NetworkFetch => "network.fetch",
            Self::DialogOpenFile => "dialog.open_file",
            Self::DialogOpenFolder => "dialog.open_folder",
            Self::FolderReadEntries => "folder.read_entries",
            Self::DialogSaveFile => "dialog.save_file",
            Self::FileReadText => "file.read_text",
            Self::FileWriteText => "file.write_text",
            Self::FileWriteBinary => "file.write_binary",
            Self::StorageStateRead => "storage.state.read",
            Self::StorageStateReplace => "storage.state.replace",
            Self::StorageStateClear => "storage.state.clear",
            Self::CredentialRead => "credential.read",
            Self::CredentialWrite => "credential.write",
            Self::CredentialDelete => "credential.delete",
            Self::NotificationShow => "notification.show",
            Self::WindowTitle => "window.title",
            Self::WindowState => "window.state",
            Self::WindowStateRead => "window.state.read",
            Self::WindowStateObserve => "window.state.observe",
            Self::WindowFocus => "window.focus",
            Self::WindowFullscreen => "window.fullscreen",
            Self::WindowSize => "window.size",
            Self::WindowOpen => "window.open",
            Self::WindowClose => "window.close",
            Self::UiFieldsRead => "ui.fields.read",
            Self::MenuWrite => "menu.write",
            Self::ContextMenuWrite => "menu.context.write",
            Self::TrayWrite => "tray.write",
        }
    }
}

/// One stable public failure code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolErrorCode {
    /// The host did not grant the required capability.
    CapabilityDenied,
    /// The request operation is not known by this protocol version.
    OperationUnsupported,
    /// The request protocol version is incompatible with the host.
    ProtocolVersionUnsupported,
    /// A transport control prevented a request from starting.
    RequestCancelled,
    /// The request envelope is malformed.
    RequestInvalid,
    /// The request payload does not meet its operation contract.
    RequestPayloadInvalid,
    /// The clipboard service was unavailable.
    ClipboardUnavailable,
    /// Clipboard text failed validation.
    ClipboardTextInvalid,
    /// Clipboard text exceeded its fixed size bound.
    ClipboardTextTooLarge,
    /// The external-link service was unavailable.
    ExternalUnavailable,
    /// The host has no authorized text-fetch service, the origin is not
    /// allowed, or the direct native request could not complete.
    NetworkUnavailable,
    /// A native response could not be represented as the bounded public
    /// status-and-UTF-8-text value.
    NetworkResponseInvalid,
    /// The file-dialog service was unavailable.
    DialogUnavailable,
    /// A selected folder reference was unavailable or unsafe to enumerate.
    FolderUnavailable,
    /// A selected file or output reference was unavailable.
    FileUnavailable,
    /// Selected file text failed validation.
    FileTextInvalid,
    /// Selected file text exceeded its fixed size bound.
    FileTextTooLarge,
    /// Canonical decoded binary output exceeded its fixed request bound.
    FileBinaryTooLarge,
    /// The storage service was unavailable.
    StorageUnavailable,
    /// A storage snapshot failed validation.
    StorageSnapshotInvalid,
    /// A storage snapshot exceeded its fixed size bound.
    StorageSnapshotTooLarge,
    /// The diagnostics service was unavailable.
    DiagnosticsUnavailable,
    /// The credential service was unavailable.
    CredentialUnavailable,
    /// Credential access was denied.
    CredentialAccessDenied,
    /// A stored credential secret could not be represented safely.
    CredentialStoredSecretInvalid,
    /// The host cannot show notifications, or the system refused. This never
    /// distinguishes a muted application from a busy shell.
    NotificationUnavailable,
    /// Another notification for this session is still pending.
    NotificationBusy,
    /// The supplied title or body failed the documented bounds or character
    /// rules. The failure never echoes the offending text back.
    NotificationTextInvalid,
    /// This session has no host window to title, or the native call failed.
    ///
    /// One code for both, deliberately: which it is describes host state an
    /// application has no business learning.
    WindowUnavailable,
    /// Another window-title proposal for this session is still pending.
    WindowBusy,
    /// The proposed title failed the documented bounds or character rules. The
    /// failure never echoes the offending text back.
    WindowTitleInvalid,
    /// This session has no surface whose field values can be read.
    ///
    /// One code for every reason. Distinguishing "no surface" from "no fields"
    /// from "the host was busy" would report state that, read repeatedly,
    /// describes what the person is doing.
    UiFieldsUnavailable,
    /// The host has no session-owned native menu, or could not update it.
    ///
    /// This intentionally does not distinguish absent UI state, a busy UI
    /// thread, or an operating-system failure.
    MenuUnavailable,
    /// The session has no host-owned notification-area tray surface, or that
    /// surface could not apply its bounded semantic model.
    ///
    /// This does not distinguish a missing session window, a busy host UI
    /// thread, or an operating-system rejection because those distinctions
    /// would expose host state to the application.
    TrayUnavailable,
}

impl ProtocolErrorCode {
    /// Returns the stable protocol spelling of this failure code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityDenied => "capability.denied",
            Self::OperationUnsupported => "operation.unsupported",
            Self::ProtocolVersionUnsupported => "protocol.version_unsupported",
            Self::RequestCancelled => "request.cancelled",
            Self::RequestInvalid => "request.invalid",
            Self::RequestPayloadInvalid => "request.payload_invalid",
            Self::ClipboardUnavailable => "clipboard.unavailable",
            Self::ClipboardTextInvalid => "clipboard.text_invalid",
            Self::ClipboardTextTooLarge => "clipboard.text_too_large",
            Self::ExternalUnavailable => "external.unavailable",
            Self::NetworkUnavailable => "network.unavailable",
            Self::NetworkResponseInvalid => "network.response_invalid",
            Self::DialogUnavailable => "dialog.unavailable",
            Self::FolderUnavailable => "folder.unavailable",
            Self::FileUnavailable => "file.unavailable",
            Self::FileTextInvalid => "file.text_invalid",
            Self::FileTextTooLarge => "file.text_too_large",
            Self::FileBinaryTooLarge => "file.binary_too_large",
            Self::StorageUnavailable => "storage.unavailable",
            Self::StorageSnapshotInvalid => "storage.snapshot_invalid",
            Self::StorageSnapshotTooLarge => "storage.snapshot_too_large",
            Self::DiagnosticsUnavailable => "diagnostics.unavailable",
            Self::CredentialUnavailable => "credential.unavailable",
            Self::CredentialAccessDenied => "credential.access_denied",
            Self::CredentialStoredSecretInvalid => "credential.stored_secret_invalid",
            Self::NotificationUnavailable => "notification.unavailable",
            Self::NotificationBusy => "notification.busy",
            Self::NotificationTextInvalid => "notification.text_invalid",
            Self::WindowUnavailable => "window.unavailable",
            Self::WindowBusy => "window.busy",
            Self::WindowTitleInvalid => "window.title_invalid",
            Self::UiFieldsUnavailable => "ui.fields.unavailable",
            Self::MenuUnavailable => "menu.unavailable",
            Self::TrayUnavailable => "tray.unavailable",
        }
    }
}
