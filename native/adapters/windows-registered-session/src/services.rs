//! Identity-bound service composition for registered Windows sessions.

use anodrel_application::InstalledApplication;
use anodrel_core::HostServices;
use anodrel_file_access::{SaveFileDialogMailbox, SelectionFileDialogMailbox};
use anodrel_folder_access::FolderFileDialogMailbox;
use anodrel_windows_clipboard::WindowsClipboard;
use anodrel_windows_credentials::WindowsCredentialService;
use anodrel_windows_external_links::WindowsExternalLinks;
use anodrel_windows_network::WindowsNetworkTextService;
use anodrel_windows_paths::application_directories;
use anodrel_windows_storage::WindowsStorageService;

use crate::{RegisteredSessionError, RegisteredSessionUi};

pub(super) fn registered_services(
    application: &InstalledApplication,
) -> Result<HostServices, RegisteredSessionError> {
    let directories = application_directories(application.identity())
        .map_err(RegisteredSessionError::Directories)?;
    let services = HostServices::unavailable()
        .with_clipboard(WindowsClipboard::new(0))
        .with_external_links(WindowsExternalLinks)
        .with_storage(WindowsStorageService::new(&directories))
        .with_credentials(WindowsCredentialService::new(
            application.identity().clone(),
        ));
    // The direct WinHTTP adapter receives only a policy that was parsed from
    // the trusted installed record. No installed session has a network service
    // merely because it carries another capability or a package requests one.
    Ok(match application.network_origin_policy() {
        Some(policy) => services.with_network(WindowsNetworkTextService::new(policy.clone())),
        None => services,
    })
}

pub(super) fn registered_interactive_services(
    application: &InstalledApplication,
    ui: &RegisteredSessionUi,
) -> Result<HostServices, RegisteredSessionError> {
    Ok(registered_services(application)?
        .with_file_dialogs(ui.file_dialog_mailbox())
        .with_file_selections(SelectionFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_file_text(ui.file_text_service())
        .with_folder_selections(FolderFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_folder_entries(ui.folder_entry_service())
        .with_file_save_selections(SaveFileDialogMailbox::new(ui.file_dialog_mailbox()))
        .with_file_text_write(ui.file_text_service().write_service())
        .with_file_binary_write(ui.file_text_service().binary_write_service())
        // Notifications reach Shell32 through the owning UI thread, so the
        // session gets the mailbox rather than the adapter.
        .with_notifications(ui.notification_mailbox())
        // A complete semantic menu reaches User32 only through this session's
        // owning UI thread; no pipe worker gains a native menu handle.
        .with_menu(ui.menu_mailbox())
        // Context menus use their own capability, mailbox, and local User32
        // popup route. The service carries no target, coordinate, or handle.
        .with_context_menu(ui.context_menu_mailbox())
        // The tray shares the session window's host-owned notification-area
        // entry and keeps Shell32 and User32 work off the pipe worker.
        .with_tray(ui.tray_mailbox())
        // A window caption reaches User32 the same way, and the UI thread holds
        // the validated display name it composes with.
        .with_window_title(ui.window_title_mailbox())
        // A presentation state takes the same host-only UI-thread path and is
        // still resolved from this session rather than a caller-supplied target.
        .with_window_state(ui.window_state_mailbox())
        // Pull-only observation uses a distinct bridge and remains unavailable
        // unless the installed record explicitly grants window.state.read.
        .with_window_state_read(ui.window_state_read_mailbox())
        // Coalesced state changes have their own policy grant and retain only
        // one latest portable value for this host-resolved session window.
        .with_window_state_changes(ui.window_state_changes_mailbox())
        // Foregrounding stays in the same session-owned UI-thread boundary.
        // The policy parser admits this mailbox only for record version 1.9.
        .with_window_focus(ui.window_focus_mailbox())
        // Reversible fullscreen uses a distinct session-local bridge. The
        // parser admits this mailbox only for record version 1.10.
        .with_window_fullscreen(ui.window_fullscreen_mailbox())
        // Bounded client sizing stays on the same session-local UI-thread
        // boundary. The parser admits this mailbox only for record version 1.12.
        .with_window_size(ui.window_size_mailbox())
        // Protocol 1.25 window.open/window.close routes use the separately
        // supplied UiWindowGroup at core construction. They need no service
        // mailbox here: the group owns only logical identities while the
        // Windows host retains its private native-window mapping.
        // Field values live with the window that owns them, so a read crosses
        // to the UI thread the same way. See `docs/UI_FIELDS.md`.
        .with_ui_fields(ui.field_mailbox()))
}
