//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use std::sync::Arc;

use anodrel_canvas::Point;
use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequest, FileDialogSelection};
use anodrel_menu::{MenuMailbox, MenuRequest};
use anodrel_notifications::{NotificationMailbox, NotificationRequest};
use anodrel_ui::{ElementId, Status, UiEvent};
use anodrel_ui_session::{
    UiDocumentMailbox, UiDocumentRevision, UiFieldMailbox, UiFieldRequest, UiInputCandidate,
    UiInputMailbox, UiWindowId, UiWindowResources,
};
use anodrel_window::{
    WindowFocusMailbox, WindowFullscreenMailbox, WindowFullscreenMode, WindowSize,
    WindowSizeMailbox, WindowState, WindowStateMailbox, WindowTitleMailbox,
};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_notifications::WindowsNotifications;
use anodrel_windows_product_session::RunningProductSession;

mod bridges;

use super::{
    Hwnd, Lparam, Wparam,
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
    notifications: NotificationMailbox,
    /// This session's one-request menu replacement bridge, when it has one.
    ///
    /// It transfers only a validated complete semantic model. The native menu
    /// objects and their private command IDs stay in this view on the UI thread.
    menu_mailbox: Option<MenuMailbox>,
    /// The currently attached native menu and its private command mapping.
    menu_bar: Option<MenuBar>,
    /// This session's notification-area entry, created the first time it
    /// actually shows something.
    ///
    /// Creating it eagerly would put an icon in the notification area for every
    /// session window, including diagnostics that never notify. The entry lives
    /// as long as the view, so it is shared rather than recreated.
    notification_entry: Option<Arc<WindowsNotifications>>,
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
            notifications,
            menu_mailbox: None,
            menu_bar: None,
            notification_entry: None,
            window_title: None,
            window_state: None,
            window_focus: None,
            window_fullscreen: None,
            window_size: None,
            fullscreen_restore: None,
            display_name: None,
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
            notifications,
            window_title,
            display_name,
            menu,
            window_state,
            window_focus,
            window_fullscreen,
            window_size,
            field_reads,
        ) = {
            let ui = session.ui();
            (
                ui.document_mailbox(),
                ui.input_mailbox(),
                ui.close_signal(),
                ui.file_dialog_mailbox(),
                ui.file_text_service(),
                ui.notification_mailbox(),
                ui.window_title_mailbox(),
                ui.display_name().to_owned(),
                ui.menu_mailbox(),
                ui.window_state_mailbox(),
                ui.window_focus_mailbox(),
                ui.window_fullscreen_mailbox(),
                ui.window_size_mailbox(),
                ui.field_mailbox(),
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
        .with_window_title(window_title, display_name)
        .with_menu(menu)
        .with_window_state(window_state)
        .with_window_focus(window_focus)
        .with_window_fullscreen(window_fullscreen)
        .with_window_size(window_size)
        .with_field_reads(field_reads)
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

    /// Updates hover state through this view's current native layout.
    pub(super) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.update_hover(width, height, at)
    }

    /// Clears hover state when the native pointer leaves this view.
    pub(super) fn clear_hover(&mut self) -> bool {
        self.lab.clear_hover()
    }

    /// Moves focus through this view's current visible actions.
    pub(super) fn focus_next(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_next(width, height)
    }

    /// Moves focus backwards through this view's current visible actions.
    pub(super) fn focus_previous(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_previous(width, height)
    }

    /// Attaches this session's field-read bridge.
    #[must_use]
    pub(super) fn with_field_reads(mut self, mailbox: UiFieldMailbox) -> Self {
        self.field_reads = Some(mailbox);
        self
    }

    /// Takes one pending field read, if this session has a bridge and a read.
    pub(super) fn take_field_read(&self) -> Option<u64> {
        self.field_reads.as_ref()?.take().map(UiFieldRequest::id)
    }

    /// Answers one field read with this view's current values.
    ///
    /// A view whose snapshot cannot be built answers unavailable rather than a
    /// partial one: a read reports the surface as it is or not at all.
    pub(super) fn complete_field_read(&self, request_id: u64) -> bool {
        let Some(mailbox) = self.field_reads.as_ref() else {
            return false;
        };
        match self.lab.field_snapshot() {
            Some(snapshot) => mailbox.complete(request_id, snapshot),
            None => mailbox.fail(request_id),
        }
    }

    /// Applies one typed character to this view's focused field.
    ///
    /// The text stays in this view. Nothing here reaches the session's mailbox,
    /// so an application learns nothing from a person typing. See
    /// `docs/UI_FIELDS.md`.
    pub(super) fn type_character(&mut self, width: f32, height: f32, character: char) -> bool {
        self.lab.type_character(width, height, character)
    }

    /// Applies one editing key to this view's focused field.
    pub(super) fn edit_focused_field(
        &mut self,
        width: f32,
        height: f32,
        edit: super::ui_lab::FieldEdit,
    ) -> bool {
        self.lab.edit_focused_field(width, height, edit)
    }

    /// Moves a current v2 scroll viewport by one local native page.
    pub(super) fn scroll_page(&mut self, width: f32, height: f32, forward: bool) -> bool {
        self.lab.scroll_page(width, height, forward)
    }

    /// Converts one native wheel delta into local owned line movement.
    pub(super) fn scroll_wheel_delta(&mut self, width: f32, height: f32, delta: i32) -> bool {
        self.lab.scroll_wheel_delta(width, height, delta)
    }

    /// Clamps retained local viewport positions after a native size change.
    pub(super) fn clamp_scroll_offsets(&mut self, width: f32, height: f32) {
        self.lab.clamp_scroll_offsets(width, height);
    }

    /// Begins one private host-local scrollbar thumb drag.
    pub(super) fn begin_scrollbar_drag(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.begin_scrollbar_drag(width, height, at)
    }

    /// Applies one captured private pointer position to a local scrollbar.
    pub(super) fn drag_scrollbar(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.drag_scrollbar(width, height, at)
    }

    /// Stops a private host-local scrollbar thumb drag.
    pub(super) fn end_scrollbar_drag(&mut self) -> bool {
        self.lab.end_scrollbar_drag()
    }

    /// Pages one local scrollbar track without queuing a semantic action.
    pub(super) fn page_scrollbar_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.page_scrollbar_at(width, height, at)
    }

    /// Queues one current pointer-derived semantic action candidate.
    pub(super) fn invoke(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(event) = self.lab.event_at(width, height, at) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Moves focus to whatever focusable item is under a pointer position.
    pub(super) fn focus_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.focus_at(width, height, at)
    }

    /// Queues one current focused semantic action candidate.
    pub(super) fn activate_focused(&mut self, width: f32, height: f32) -> bool {
        let Some(event) = self.lab.focused_event(width, height) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Returns whether the current layout has a hovered action.
    pub(super) fn is_hovered(&self) -> bool {
        self.lab.hovered.is_some()
    }

    /// Returns the local native renderer state.
    pub(super) const fn lab(&self) -> &UiLab {
        &self.lab
    }

    /// Returns the bounded semantic action route for the current document.
    ///
    /// UI Automation receives only this small immutable route, never this
    /// view, the window registry, or a native handle. An initial session has no
    /// document revision to bind an action to and therefore supplies none.
    pub(super) fn accessibility_action_sink(
        &self,
    ) -> Option<anodrel_windows_uia::UiAutomationActionSink> {
        anodrel_windows_uia::UiAutomationActionSink::for_current_session(
            self.revision,
            self.input_mailbox.clone(),
        )
    }

    /// Binds this current session revision to its host-only UIA focus route.
    pub(super) fn accessibility_focus_route(&self) -> anodrel_windows_uia::UiAutomationFocusRoute {
        self.lab.accessibility_focus_route(Some(self.revision))
    }

    /// Binds this current session revision to its host-only UIA scroll route.
    pub(super) fn accessibility_scroll_route(
        &self,
    ) -> anodrel_windows_uia::UiAutomationScrollRoute {
        self.lab.accessibility_scroll_route(Some(self.revision))
    }

    /// Services one UI Automation focus request on the owning UI thread.
    pub(super) fn service_accessibility_focus(
        &mut self,
        width: f32,
        height: f32,
    ) -> Option<AccessibilityFocusResult> {
        self.lab
            .service_accessibility_focus(Some(self.revision), width, height)
    }

    /// Services one UI Automation scroll request on the owning UI thread.
    pub(super) fn service_accessibility_scroll(
        &mut self,
        width: f32,
        height: f32,
    ) -> Option<AccessibilityScrollResult> {
        self.lab
            .service_accessibility_scroll(Some(self.revision), width, height)
    }

    fn queue_event(&self, event: UiEvent) -> bool {
        if self.revision == UiDocumentRevision::INITIAL {
            return false;
        }
        self.input_mailbox
            .push(UiInputCandidate::new(self.revision, event));
        true
    }
}

#[cfg(test)]
mod tests;
