//! One host-controlled native consumer of a bounded authenticated UI mailbox.

use std::sync::Arc;

use anodrel_canvas::Point;
use anodrel_core::SessionCloseSignal;
use anodrel_file_dialog::{FileDialogMailbox, FileDialogRequest, FileDialogSelection};
use anodrel_ui::UiEvent;
use anodrel_ui_session::{UiDocumentMailbox, UiDocumentRevision, UiInputCandidate, UiInputMailbox};
use anodrel_windows_file_access::WindowsFileTextService;
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
    revision: UiDocumentRevision,
    /// The product session whose resources this view consumes, when the window
    /// owns one.
    ///
    /// Holding it here ties the verified child, pipe worker, and exit watcher to
    /// this window's lifetime: removing the view when the window is destroyed
    /// drops the last reference, and `RunningProductSession` requests full
    /// shutdown on drop. A diagnostic session view holds `None`.
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
    ) -> Self {
        Self {
            lab: UiLab::waiting_for_session(),
            mailbox,
            input_mailbox,
            close_signal,
            file_dialog_mailbox,
            file_text,
            revision: UiDocumentRevision::INITIAL,
            product_session: None,
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
        );
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
    use anodrel_ui::UiEvent;
    use anodrel_ui_session::{UiDocumentMailbox, UiDocumentSession, UiInputMailbox};
    use anodrel_windows_file_access::WindowsFileTextService;

    use super::UiSessionView;

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
    fn consumes_only_its_supplied_session_close_signal() {
        let signal = SessionCloseSignal::default();
        let mut view = UiSessionView::new(
            UiDocumentMailbox::new(),
            UiInputMailbox::new(),
            signal.clone(),
            FileDialogMailbox::new(),
            WindowsFileTextService::new(),
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
