//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use std::sync::Arc;

use anodrel_canvas::Point;
use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequest, FileDialogSelection};
use anodrel_menu::{
    ContextMenuMailbox, ContextMenuRequest, MenuMailbox, MenuRequest, TrayMailbox, TrayRequest,
};
use anodrel_notifications::{NotificationMailbox, NotificationRequest};
use anodrel_ui::{ElementId, Status, UiEvent};
use anodrel_ui_session::{
    UiDocumentMailbox, UiDocumentRevision, UiFieldMailbox, UiFieldRequest, UiInputCandidate,
    UiInputMailbox, UiWindowId, UiWindowResources,
};
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowFullscreenMode, WindowSize,
    WindowSizeMailbox, WindowState, WindowStateChangesMailbox, WindowStateMailbox,
    WindowStateReadMailbox, WindowTitleMailbox,
};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_folder_access::WindowsFolderEntryService;
use anodrel_windows_notification_area::NotificationArea;
use anodrel_windows_product_session::RunningProductSession;

mod bridges;
mod interaction;
mod tray_bridge;

use super::{
    Hwnd, Lparam, Wparam,
    context_menu::ContextMenu,
    menu::{MenuBar, UnattachedMenu},
    session_window_group::{SessionWindowMember, SessionWindowOpenRequest},
    ui_lab::{AccessibilityFocusResult, AccessibilityScrollResult, UiLab},
};

/// A native session view with no application input or event delivery.
#[derive(Clone)]
pub(super) struct UiSessionView {
    lab: UiLab,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    /// This session's retained selected-folder entry registry, when its core
    /// services expose the Protocol 1.29 one-use route.
    folder_entries: Option<WindowsFolderEntryService>,
    notifications: NotificationMailbox,
    /// This session's one-request menu replacement bridge, when it has one.
    ///
    /// It transfers only a validated complete semantic model. The native menu
    /// objects and their private command IDs stay in this view on the UI thread.
    menu_mailbox: Option<MenuMailbox>,
    /// The currently attached native menu and its private command mapping.
    menu_bar: Option<MenuBar>,
    /// This session's one-request context-menu replacement bridge, when it has one.
    context_menu_mailbox: Option<ContextMenuMailbox>,
    /// The current host-retained context-menu model and private command mapping.
    context_menu: Option<ContextMenu>,
    /// This session's one-request notification-area tray bridge, when it has one.
    tray_mailbox: Option<TrayMailbox>,
    /// The current host-retained tray model and private command mapping.
    tray: Option<super::tray::TrayMenu>,
    /// This session's notification-area entry, created the first time it
    /// actually shows something.
    ///
    /// Creating it eagerly would put an icon in the notification area for every
    /// session window, including diagnostics that never notify. The entry lives
    /// as long as the view, so it is shared rather than recreated.
    notification_entry: Option<Arc<NotificationArea>>,
    /// This session's one-request window-title bridge, when it has one.
    ///
    /// A diagnostic session view holds `None` and answers every proposal as
    /// unavailable, which is the same thing an application would see on a host
    /// with no window to title.
    window_title: Option<WindowTitleMailbox>,
    /// This session's one-request presentation-state bridge, when it has one.
    ///
    /// The value is a closed portable enum. The view supplies no target or
    /// handle, so the host UI thread can apply it only to its own window.
    window_state: Option<WindowStateMailbox>,
    /// This session's pull-only presentation-state observation bridge.
    ///
    /// It carries no target and can answer only with one immediate closed
    /// state. It intentionally has no listener, event, geometry, or focus
    /// surface; see `docs/WINDOW_STATE_OBSERVATION.md`.
    window_state_read: Option<WindowStateReadMailbox>,
    /// This session's coalesced pull-only presentation-change mailbox.
    ///
    /// It retains at most one later closed state. No timing, history, wait,
    /// callback, or subscription reaches this view or its application.
    window_state_changes: Option<WindowStateChangesMailbox>,
    /// This session's one-request foreground bridge, when it has one.
    ///
    /// The view supplies no target or native handle. Its owning UI thread can
    /// ask Windows to foreground only the window that owns this view.
    window_focus: Option<WindowFocusMailbox>,
    /// This session's one-request reversible fullscreen bridge, when it has
    /// one. The host keeps its saved native presentation facts beside this
    /// session view; neither facts nor a native handle reach the application.
    window_fullscreen: Option<WindowFullscreenMailbox>,
    /// This session's one-request bounded client-size bridge, when it has one.
    ///
    /// It transfers only a validated logical client size. The owning UI thread
    /// retains its DPI, menu, frame, and outer geometry calculations.
    window_size: Option<WindowSizeMailbox>,
    /// The pre-fullscreen native presentation facts retained by the host.
    ///
    /// `Some` means the host has a restoration path it must preserve. It is
    /// never a protocol-visible current-state flag.
    fullscreen_restore: Option<super::fullscreen::FullscreenRestore>,
    /// The validated display name appended to any title this session proposes.
    ///
    /// Held beside the mailbox rather than passed with each request: it comes
    /// from the installed record, not from the application, and keeping it on
    /// this side is what makes the suffix impossible to influence.
    display_name: Option<String>,
    /// The machine-validated application identity that may expose the fixed
    /// native update system-menu action.
    ///
    /// It is copied only into private registry metadata, never to an
    /// application, UI document, protocol message, menu model, or renderer.
    product_update_application_id: Option<String>,
    /// This session's field-read bridge, when it has one.
    ///
    /// A diagnostic session view holds `None` and answers every read as
    /// unavailable, which is what an application sees on a host with no surface
    /// it may read.
    field_reads: Option<UiFieldMailbox>,
    revision: UiDocumentRevision,
    /// The semantic status from the last accepted session document.
    ///
    /// It establishes the event baseline only; it is not a delivery record,
    /// listener state, or application-visible value.
    last_status: Option<Status>,
    /// The private membership of this view in a native session-window group.
    ///
    /// It is absent for the legacy one-window diagnostic. When present, it
    /// keeps the verified product lifetime with the group rather than this one
    /// view and lets the UI thread poll group-wide close and open handoffs.
    session_window: Option<SessionWindowMember>,
}

/// The one host-only outcome from polling a session document mailbox.
///
/// A status ID means only that a later accepted document changed its declared
/// status. The window procedure still verifies visible current publication
/// before it raises any outbound Windows event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct UiSessionPoll {
    pub(super) document_changed: bool,
    pub(super) close_requested: bool,
    pub(super) changed_status: Option<ElementId>,
}

impl UiSessionView {
    /// Creates the host-owned waiting surface for one supplied session mailbox.
    pub(super) fn new(
        mailbox: UiDocumentMailbox,
        input_mailbox: UiInputMailbox,
        close_signal: SessionCloseSignal,
        file_dialog_mailbox: FileDialogMailbox,
        file_text: WindowsFileTextService,
        notifications: NotificationMailbox,
    ) -> Self {
        Self {
            lab: UiLab::waiting_for_session(),
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            folder_entries: None,
            notifications,
            menu_mailbox: None,
            menu_bar: None,
            context_menu_mailbox: None,
            context_menu: None,
            tray_mailbox: None,
            tray: None,
            notification_entry: None,
            window_title: None,
            window_state: None,
            window_state_read: None,
            window_state_changes: None,
            window_focus: None,
            window_fullscreen: None,
            window_size: None,
            fullscreen_restore: None,
            display_name: None,
            product_update_application_id: None,
            field_reads: None,
            revision: UiDocumentRevision::INITIAL,
            last_status: None,
            session_window: None,
        }
    }

    /// Creates the view for one verified product session and takes ownership of
    /// its lifetime.
    ///
    /// Every resource comes from that same session's group, so this window can
    /// never poll another session's mailbox.
    pub(super) fn for_product_session(session: RunningProductSession) -> Self {
        let (
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            folder_entries,
            notifications,
            window_title,
            display_name,
            menu,
            context_menu,
            tray,
            window_state,
            window_state_read,
            window_state_changes,
            window_focus,
            window_fullscreen,
            window_size,
            field_reads,
            product_update_application_id,
        ) = {
            let ui = session.ui();
            (
                ui.document_mailbox(),
                ui.input_mailbox(),
                ui.close_signal(),
                ui.file_dialog_mailbox(),
                ui.file_text_service(),
                ui.folder_entry_service(),
                ui.notification_mailbox(),
                ui.window_title_mailbox(),
                ui.display_name().to_owned(),
                ui.menu_mailbox(),
                ui.context_menu_mailbox(),
                ui.tray_mailbox(),
                ui.window_state_mailbox(),
                ui.window_state_read_mailbox(),
                ui.window_state_changes_mailbox(),
                ui.window_focus_mailbox(),
                ui.window_fullscreen_mailbox(),
                ui.window_size_mailbox(),
                ui.field_mailbox(),
                ui.update_application_id().map(str::to_owned),
            )
        };
        let group = super::session_window_group::SessionWindowGroup::for_product_session(session);
        Self::new(
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            notifications,
        )
        .with_folder_entries(folder_entries)
        .with_window_title(window_title, display_name)
        .with_menu(menu)
        .with_context_menu(context_menu)
        .with_tray(tray)
        .with_window_state(window_state)
        .with_window_state_read(window_state_read)
        .with_window_state_changes(window_state_changes)
        .with_window_focus(window_focus)
        .with_window_fullscreen(window_fullscreen)
        .with_window_size(window_size)
        .with_field_reads(field_reads)
        .with_product_update_application_id(product_update_application_id)
        .with_session_window(group.member(UiWindowId::primary()))
    }

    /// Creates the primary native view for the fixed development multi-window
    /// route. The caller has already bound the same portable resources into the
    /// authenticated pipe core and private native group. Empty bridge values
    /// deliberately leave every unrelated service unavailable.
    pub(super) fn for_group_primary(
        resources: UiWindowResources,
        close_signal: SessionCloseSignal,
        session_window: SessionWindowMember,
    ) -> Self {
        Self::new(
            resources.document_mailbox(),
            resources.input_mailbox(),
            close_signal,
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        )
        .with_session_window(session_window)
    }

    /// Creates the deliberately limited native view for one group member.
    ///
    /// It receives only its own document mailbox, semantic-input mailbox, and
    /// group-close state. New empty bridges are private unavailable sentinels,
    /// not inherited services: no pipe route references them, so a secondary
    /// cannot use a primary dialog, file, notification, menu, title, state,
    /// focus, fullscreen, size, or field-read bridge.
    pub(super) fn for_group_member(
        resources: UiWindowResources,
        session_window: SessionWindowMember,
    ) -> Self {
        Self::new(
            resources.document_mailbox(),
            resources.input_mailbox(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        )
        .with_session_window(session_window)
    }

    /// Attaches one host-only native-group member to this view.
    #[must_use]
    fn with_session_window(mut self, session_window: SessionWindowMember) -> Self {
        self.session_window = Some(session_window);
        self
    }

    /// Keeps one signed-policy-derived update identity in this product view's
    /// private registry metadata. `None` is the ordinary no-catalogue case and
    /// must not create a native update action.
    #[must_use]
    fn with_product_update_application_id(mut self, application_id: Option<String>) -> Self {
        self.product_update_application_id = application_id;
        self
    }

    /// Returns the registry-only product-update identity, when signed policy
    /// selected a catalogue for this exact authenticated product session.
    pub(in crate::win32) fn product_update_application_id(&self) -> Option<&str> {
        self.product_update_application_id.as_deref()
    }

    /// Connects this view to its same-session retained-folder registry.
    #[must_use]
    pub(super) fn with_folder_entries(mut self, service: WindowsFolderEntryService) -> Self {
        self.folder_entries = Some(service);
        self
    }

    /// Applies at most one newer accepted snapshot from this view's mailbox.
    pub(super) fn poll(&mut self) -> UiSessionPoll {
        let close_requested = self.session_window.as_ref().map_or_else(
            || self.close_signal.take(),
            SessionWindowMember::observe_shutdown,
        );
        let Some(snapshot) = self.mailbox.take() else {
            return UiSessionPoll {
                document_changed: false,
                close_requested,
                changed_status: None,
            };
        };
        if snapshot.revision() <= self.revision {
            return UiSessionPoll {
                document_changed: false,
                close_requested,
                changed_status: None,
            };
        }
        let established = self.revision != UiDocumentRevision::INITIAL;
        let next_status = snapshot.document().status().cloned();
        let changed_status = if established && next_status.as_ref() != self.last_status.as_ref() {
            next_status.as_ref().map(|status| status.id().clone())
        } else {
            None
        };
        self.revision = snapshot.revision();
        self.lab.replace_document(snapshot.document().clone());
        self.last_status = next_status;
        UiSessionPoll {
            document_changed: true,
            close_requested,
            changed_status,
        }
    }

    /// Registers this view's host-owned logical identity before the window is
    /// shown. Legacy diagnostic views return `false` because they are not part
    /// of an authenticated session group.
    pub(super) fn register_native_window(&self, window: Hwnd) -> bool {
        self.session_window
            .as_ref()
            .is_some_and(|member| member.register_native_window(window))
    }

    /// Returns whether this view belongs to a session-owned native group.
    pub(super) const fn is_group_member(&self) -> bool {
        self.session_window.is_some()
    }

    /// Removes this view's native mapping after Windows has destroyed it.
    ///
    /// This runs only for the real view removed from the registry, never for a
    /// paint snapshot clone. See `SessionWindowMember::on_native_destroy`.
    pub(super) fn on_native_destroy(&self, window: Hwnd) {
        if let Some(member) = &self.session_window {
            member.on_native_destroy(window);
        }
    }

    /// Takes one secondary creation handoff for this group, if this view belongs
    /// to one. The host creates and registers the resulting native window on
    /// this same UI thread before it completes the portable request.
    pub(super) fn take_secondary_open_request(&self) -> Option<SessionWindowOpenRequest> {
        self.session_window
            .as_ref()
            .and_then(SessionWindowMember::take_open_request)
    }

    /// Takes host-private native windows requested for secondary-view close.
    ///
    /// The member resolves only identities belonging to this authenticated
    /// group. A view without a group has no cross-view close route at all.
    pub(super) fn take_secondary_close_windows(&self) -> Vec<Hwnd> {
        self.session_window
            .as_ref()
            .map_or_else(Vec::new, SessionWindowMember::take_secondary_close_windows)
    }

    /// Takes a pending modal request for the host UI thread.
    pub(super) fn take_file_dialog_request(&self) -> Option<FileDialogRequest> {
        self.file_dialog_mailbox.take()
    }

    /// Completes a modal request after the host UI thread returns from Windows.
    pub(super) fn complete_file_dialog_request(
        &self,
        request_id: u64,
        selection: Result<FileDialogSelection, anodrel_windows_file_dialog::FileDialogError>,
    ) -> bool {
        match selection {
            Ok(selection) => self.file_dialog_mailbox.complete(request_id, selection),
            Err(_) => self.file_dialog_mailbox.fail(request_id),
        }
    }

    /// Returns this view's session-bound retained-file registry for one UI
    /// selection capture. The clone shares only session-local native state.
    pub(super) fn file_text_service(&self) -> WindowsFileTextService {
        self.file_text.clone()
    }

    /// Returns this view's retained selected-folder registry when configured.
    pub(super) fn folder_entry_service(&self) -> Option<WindowsFolderEntryService> {
        self.folder_entries.clone()
    }
}

#[cfg(test)]
mod tests;
