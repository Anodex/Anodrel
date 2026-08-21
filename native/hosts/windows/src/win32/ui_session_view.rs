//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use std::sync::Arc;

use anodrel_canvas::Point;
use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequest, FileDialogSelection};
use anodrel_notifications::{NotificationMailbox, NotificationRequest};
use anodrel_ui::UiEvent;
use anodrel_ui_session::{
    UiDocumentMailbox, UiDocumentRevision, UiFieldMailbox, UiFieldRequest, UiInputCandidate,
    UiInputMailbox,
};
use anodrel_window::{WindowState, WindowStateMailbox, WindowTitleMailbox};
use anodrel_windows_file_access::WindowsFileTextService;
use anodrel_windows_notifications::WindowsNotifications;
use anodrel_windows_product_session::RunningProductSession;

use super::ui_lab::UiLab;

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
    /// The product session whose resources this view consumes, when the window
    /// owns one.
    ///
    /// Holding it here ties the verified child, pipe worker, and exit watcher to
    /// this window's lifetime: removing the view when the window is destroyed
    /// drops the last reference, and `RunningProductSession` then performs the
    /// same shutdown and worker joins its explicit `finish` would. A diagnostic
    /// session view holds `None`.
    product_session: Option<Arc<RunningProductSession>>,
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
            notification_entry: None,
            window_title: None,
            window_state: None,
            display_name: None,
            field_reads: None,
            revision: UiDocumentRevision::INITIAL,
            product_session: None,
        }
    }

    /// Attaches this session's window-title bridge and its validated name.
    ///
    /// A builder rather than another `new` parameter: the two values arrive
    /// together, only a registered session has them, and `new` already carries
    /// as many resources as one signature usefully can.
    #[must_use]
    pub(super) fn with_window_title(
        mut self,
        mailbox: WindowTitleMailbox,
        display_name: impl Into<String>,
    ) -> Self {
        self.window_title = Some(mailbox);
        self.display_name = Some(display_name.into());
        self
    }

    /// Attaches this session's closed presentation-state bridge.
    #[must_use]
    pub(super) fn with_window_state(mut self, mailbox: WindowStateMailbox) -> Self {
        self.window_state = Some(mailbox);
        self
    }

    /// Takes a pending title proposal and composes the caption to apply.
    ///
    /// Composition happens here, on the side that holds the validated name, so
    /// the value handed to User32 is never one an application chose outright.
    pub(super) fn take_window_title_request(&self) -> Option<(u64, String)> {
        let request = self.window_title.as_ref()?.take()?;
        let caption = anodrel_window::compose(request.proposal(), self.display_name.as_deref());
        Some((request.id(), caption))
    }

    /// Completes a title proposal after the host UI thread returns from User32.
    pub(super) fn complete_window_title_request(&self, request_id: u64, applied: bool) -> bool {
        let Some(mailbox) = self.window_title.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes one closed state command for this window's owning UI thread.
    pub(super) fn take_window_state_request(&self) -> Option<(u64, WindowState)> {
        let request = self.window_state.as_ref()?.take()?;
        Some((request.id(), request.state()))
    }

    /// Completes a state command after the host UI thread applies it.
    pub(super) fn complete_window_state_request(&self, request_id: u64, applied: bool) -> bool {
        let Some(mailbox) = self.window_state.as_ref() else {
            return false;
        };
        if applied {
            mailbox.complete(request_id)
        } else {
            mailbox.fail(request_id)
        }
    }

    /// Takes a pending notification for the host UI thread.
    ///
    /// The entry is returned alongside so the Shell32 call happens outside the
    /// window registry's lock.
    pub(super) fn take_notification_request(
        &self,
    ) -> Option<(NotificationRequest, Option<Arc<WindowsNotifications>>)> {
        let request = self.notifications.take()?;
        Some((request, self.notification_entry.clone()))
    }

    /// Records the entry this session created on its first notification.
    pub(super) fn set_notification_entry(&mut self, entry: Arc<WindowsNotifications>) {
        self.notification_entry = Some(entry);
    }

    /// Completes a notification after the host UI thread returns from Shell32.
    pub(super) fn complete_notification_request(&self, request_id: u64, shown: bool) -> bool {
        if shown {
            self.notifications.complete(request_id)
        } else {
            self.notifications.fail(request_id)
        }
    }

    /// Creates the view for one verified product session and takes ownership of
    /// its lifetime.
    ///
    /// Every resource comes from that same session's group, so this window can
    /// never poll another session's mailbox.
    pub(super) fn for_product_session(session: RunningProductSession) -> Self {
        let ui = session.ui();
        let mut view = Self::new(
            ui.document_mailbox(),
            ui.input_mailbox(),
            ui.close_signal(),
            ui.file_dialog_mailbox(),
            ui.file_text_service(),
            ui.notification_mailbox(),
        )
        .with_window_title(ui.window_title_mailbox(), ui.display_name())
        .with_window_state(ui.window_state_mailbox())
        .with_field_reads(ui.field_mailbox());
        view.product_session = Some(Arc::new(session));
        view
    }

    /// Applies at most one newer accepted snapshot from this view's mailbox.
    pub(super) fn poll(&mut self) -> (bool, bool) {
        let close_requested = self.close_signal.take();
        let Some(snapshot) = self.mailbox.take() else {
            return (false, close_requested);
        };
        if snapshot.revision() <= self.revision {
            return (false, close_requested);
        }
        self.revision = snapshot.revision();
        self.lab.replace_document(snapshot.document().clone());
        (true, close_requested)
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

    /// Services one UI Automation focus request on the owning UI thread.
    pub(super) fn service_accessibility_focus(&mut self, width: f32, height: f32) -> bool {
        self.lab
            .service_accessibility_focus(Some(self.revision), width, height)
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
mod tests {
    use anodrel_core::SessionCloseSignal;
    use anodrel_file_dialog::FileDialogMailbox;
    use anodrel_notifications::NotificationMailbox;
    use anodrel_ui::UiEvent;
    use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSession, UiInputMailbox};
    use anodrel_windows_file_access::WindowsFileTextService;

    use super::{UiSessionView, WindowState, WindowStateMailbox, WindowTitleMailbox};

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.root","kind":"text","value":"Connected","fontSize":16,"tone":"primary"}}"#;
    const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.action","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
    const SCROLL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v2","root":{"id":"session.viewport","kind":"scroll","child":{"id":"session.content","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"session.one","kind":"action","label":"One","fontSize":16,"enabled":true,"tone":"accent"},{"id":"session.two","kind":"action","label":"Two","fontSize":16,"enabled":true,"tone":"accent"},{"id":"session.three","kind":"action","label":"Three","fontSize":16,"enabled":true,"tone":"accent"}]}}}"#;

    #[test]
    fn applies_only_a_newer_snapshot_from_its_own_mailbox() {
        let mailbox = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(
            mailbox.clone(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        let mut session = UiDocumentSession::new();
        session
            .replace_document(DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));

        assert_eq!(view.poll(), (true, false));
        assert_eq!(view.poll(), (false, false));
    }

    #[test]
    fn accessibility_has_no_action_route_before_a_document_and_one_after() {
        let documents = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(
            documents.clone(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        assert!(view.accessibility_action_sink().is_none());

        let mut session = UiDocumentSession::new();
        session
            .replace_document(ACTION_DOCUMENT)
            .expect("document is valid");
        documents.publish(session.snapshot().expect("snapshot is available"));
        assert_eq!(view.poll(), (true, false));
        assert!(view.accessibility_action_sink().is_some());
    }

    /// A document holding one enabled field, for the read path.
    const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"session.field","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true}}"#;

    /// A session view with a title bridge and the given validated name.
    fn view_with_title(display_name: &str) -> (UiSessionView, WindowTitleMailbox) {
        let mailbox = WindowTitleMailbox::new();
        let view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        )
        .with_window_title(mailbox.clone(), display_name);
        (view, mailbox)
    }

    /// A session view with its own presentation-state bridge.
    fn view_with_state() -> (UiSessionView, WindowStateMailbox) {
        let mailbox = WindowStateMailbox::new();
        let view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        )
        .with_window_state(mailbox.clone());
        (view, mailbox)
    }

    /// Proposes a title from a worker and returns what the UI thread would apply.
    fn caption_for(view: &UiSessionView, mailbox: &WindowTitleMailbox, proposal: &str) -> String {
        let proposal =
            anodrel_window::WindowTitleProposal::new(proposal).expect("the proposal is valid");
        let worker = mailbox.clone();
        let waiting = std::thread::spawn(move || {
            anodrel_window::WindowTitleService::set_title(&worker, &proposal)
        });
        let (request_id, caption) = loop {
            if let Some(taken) = view.take_window_title_request() {
                break taken;
            }
            std::thread::yield_now();
        };
        assert!(view.complete_window_title_request(request_id, true));
        waiting
            .join()
            .expect("the worker did not panic")
            .expect("the proposal was accepted");
        caption
    }

    #[test]
    fn the_caption_a_session_applies_always_ends_with_its_validated_name() {
        // This is the impersonation guard at the point it actually matters: the
        // string handed to User32. Whatever the application proposes, the
        // caption still names the application the host validated.
        let (view, mailbox) = view_with_title("Anodrel Sample");

        assert_eq!(
            caption_for(&view, &mailbox, "Quarterly Report.pdf"),
            "Quarterly Report.pdf \u{2014} Anodrel Sample"
        );
        assert_eq!(
            caption_for(&view, &mailbox, "Windows Security"),
            "Windows Security \u{2014} Anodrel Sample"
        );
        // Even a proposal that already carries the separator cannot end the
        // caption before the real name.
        assert!(
            caption_for(&view, &mailbox, "Report \u{2014} Some Other App")
                .ends_with(" \u{2014} Anodrel Sample")
        );
    }

    #[test]
    fn a_granted_read_returns_the_text_a_person_actually_typed() {
        // The whole path in one test: a document seeds a field, a person types
        // into the host's state, and a read crossing the UI-thread bridge
        // returns exactly that text.
        let mailbox = anodrel_ui_session::UiFieldMailbox::new();
        let documents = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(
            documents.clone(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        )
        .with_field_reads(mailbox.clone());

        // Delivered the way a real session delivers one, through the mailbox.
        let mut session = UiDocumentSession::new();
        session
            .replace_document(FIELD_DOCUMENT)
            .expect("document is valid");
        documents.publish(session.snapshot().expect("snapshot is available"));
        assert_eq!(view.poll(), (true, false));

        let width = 920.0;
        let height = 660.0;
        view.focus_next(width, height);
        for character in "Ada".chars() {
            assert!(view.type_character(width, height, character));
        }

        let worker = mailbox.clone();
        let waiting = std::thread::spawn(move || anodrel_ui_session::UiFieldReader::read(&worker));
        let request_id = loop {
            if let Some(id) = view.take_field_read() {
                break id;
            }
            std::thread::yield_now();
        };
        assert!(view.complete_field_read(request_id));

        let snapshot = waiting
            .join()
            .expect("the worker did not panic")
            .expect("the read succeeded");
        assert_eq!(snapshot.fields().len(), 1);
        assert_eq!(snapshot.fields()[0].id().as_str(), "session.field");
        assert_eq!(snapshot.fields()[0].value(), "Ada");
    }

    #[test]
    fn a_session_without_a_field_bridge_answers_nothing_and_completes_nothing() {
        let view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        assert!(view.take_field_read().is_none());
        assert!(!view.complete_field_read(1));
    }

    #[test]
    fn a_session_without_a_title_bridge_answers_nothing_and_completes_nothing() {
        // The diagnostic session view has no bridge. It must not panic, and it
        // must not claim to have completed a request it never had.
        let view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        assert!(view.take_window_title_request().is_none());
        assert!(!view.complete_window_title_request(1, true));
    }

    #[test]
    fn a_session_without_a_state_bridge_answers_nothing_and_completes_nothing() {
        let view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        assert!(view.take_window_state_request().is_none());
        assert!(!view.complete_window_state_request(1, true));
    }

    #[test]
    fn one_session_cannot_take_another_sessions_state_command() {
        let (first, first_mailbox) = view_with_state();
        let (second, _second_mailbox) = view_with_state();
        let worker = first_mailbox.clone();
        let waiting = std::thread::spawn(move || {
            anodrel_window::WindowStateService::set_state(&worker, WindowState::Maximized)
        });

        let (request_id, state) = loop {
            assert!(
                second.take_window_state_request().is_none(),
                "a session took another session's state command"
            );
            if let Some(request) = first.take_window_state_request() {
                break request;
            }
            std::thread::yield_now();
        };
        assert_eq!(state, WindowState::Maximized);
        assert!(first.complete_window_state_request(request_id, true));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn one_session_cannot_take_another_sessions_title_proposal() {
        // Each view holds its own bridge, so a proposal made to one session is
        // invisible to every other window in the same message loop.
        let (first, first_mailbox) = view_with_title("First Application");
        let (second, _second_mailbox) = view_with_title("Second Application");

        let proposal =
            anodrel_window::WindowTitleProposal::new("Report").expect("the proposal is valid");
        let worker = first_mailbox.clone();
        let waiting = std::thread::spawn(move || {
            anodrel_window::WindowTitleService::set_title(&worker, &proposal)
        });
        while {
            let pending = second.take_window_title_request();
            assert!(pending.is_none(), "a session took another's proposal");
            first_mailbox.clone().take().is_none()
        } {
            std::thread::yield_now();
        }
        assert!(first.complete_window_title_request(1, false));
        assert!(waiting.join().expect("the worker did not panic").is_err());
    }

    #[test]
    fn consumes_only_its_supplied_session_close_signal() {
        let signal = SessionCloseSignal::default();
        let mut view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            signal.clone(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );

        assert_eq!(view.poll(), (false, false));
        signal.request();
        assert_eq!(view.poll(), (false, true));
        assert_eq!(view.poll(), (false, false));
    }

    #[test]
    fn queues_a_focused_action_only_with_the_current_document_revision() {
        let mailbox = UiDocumentMailbox::new();
        let inputs = UiInputMailbox::new();
        let mut view = UiSessionView::new(
            mailbox.clone(),
            inputs.clone(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        let mut session = UiDocumentSession::new();
        session
            .replace_document(ACTION_DOCUMENT)
            .expect("document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));
        assert_eq!(view.poll(), (true, false));

        assert!(view.focus_next(920.0, 660.0));
        assert!(view.activate_focused(920.0, 660.0));
        let batch = inputs.drain();
        assert_eq!(batch.dropped(), 0);
        let candidates = batch.into_candidates();
        assert_eq!(candidates.len(), 1);
        let (revision, UiEvent::ActionInvoked(action)) = candidates
            .into_iter()
            .next()
            .expect("one action candidate exists")
            .into_parts();
        assert_eq!(revision.value(), 1);
        assert_eq!(action.as_str(), "session.action");
    }

    #[test]
    fn scrolls_an_explicit_version_two_snapshot_only_in_local_view_state() {
        let mailbox = UiDocumentMailbox::new();
        let mut view = UiSessionView::new(
            mailbox.clone(),
            UiInputMailbox::new(),
            SessionCloseSignal::default(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
            NotificationMailbox::new(),
        );
        let mut session = UiDocumentSession::new();
        session
            .replace_document_v2(SCROLL_DOCUMENT)
            .expect("version two document is valid");
        mailbox.publish(session.snapshot().expect("snapshot is available"));
        assert_eq!(view.poll(), (true, false));

        assert!(view.scroll_page(920.0, 70.0, true));
        assert_eq!(view.revision.value(), 1);
    }
}
