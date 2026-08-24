//! Local input, scrolling, fields, and UI Automation handoffs for a session view.

use super::*;

impl UiSessionView {
    /// Updates hover state through this view's current native layout.
    pub(in crate::win32) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.update_hover(width, height, at)
    }

    /// Clears hover state when the native pointer leaves this view.
    pub(in crate::win32) fn clear_hover(&mut self) -> bool {
        self.lab.clear_hover()
    }

    /// Moves focus through this view's current visible actions.
    pub(in crate::win32) fn focus_next(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_next(width, height)
    }

    /// Moves focus backwards through this view's current visible actions.
    pub(in crate::win32) fn focus_previous(&mut self, width: f32, height: f32) -> bool {
        self.lab.focus_previous(width, height)
    }

    /// Attaches this session's field-read bridge.
    #[must_use]
    pub(in crate::win32) fn with_field_reads(mut self, mailbox: UiFieldMailbox) -> Self {
        self.field_reads = Some(mailbox);
        self
    }

    /// Takes one pending field read, if this session has a bridge and a read.
    pub(in crate::win32) fn take_field_read(&self) -> Option<u64> {
        self.field_reads.as_ref()?.take().map(UiFieldRequest::id)
    }

    /// Answers one field read with this view's current values.
    ///
    /// A view whose snapshot cannot be built answers unavailable rather than a
    /// partial one: a read reports the surface as it is or not at all.
    pub(in crate::win32) fn complete_field_read(&self, request_id: u64) -> bool {
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
    pub(in crate::win32) fn type_character(
        &mut self,
        width: f32,
        height: f32,
        character: char,
    ) -> bool {
        self.lab.type_character(width, height, character)
    }

    /// Applies one editing key to this view's focused field.
    pub(in crate::win32) fn edit_focused_field(
        &mut self,
        width: f32,
        height: f32,
        edit: super::super::ui_lab::FieldEdit,
    ) -> bool {
        self.lab.edit_focused_field(width, height, edit)
    }

    /// Moves a current v2 scroll viewport by one local native page.
    pub(in crate::win32) fn scroll_page(&mut self, width: f32, height: f32, forward: bool) -> bool {
        self.lab.scroll_page(width, height, forward)
    }

    /// Converts one native wheel delta into local owned line movement.
    pub(in crate::win32) fn scroll_wheel_delta(
        &mut self,
        width: f32,
        height: f32,
        delta: i32,
    ) -> bool {
        self.lab.scroll_wheel_delta(width, height, delta)
    }

    /// Clamps retained local viewport positions after a native size change.
    pub(in crate::win32) fn clamp_scroll_offsets(&mut self, width: f32, height: f32) {
        self.lab.clamp_scroll_offsets(width, height);
    }

    /// Begins one private host-local scrollbar thumb drag.
    pub(in crate::win32) fn begin_scrollbar_drag(
        &mut self,
        width: f32,
        height: f32,
        at: Point,
    ) -> bool {
        self.lab.begin_scrollbar_drag(width, height, at)
    }

    /// Applies one captured private pointer position to a local scrollbar.
    pub(in crate::win32) fn drag_scrollbar(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.drag_scrollbar(width, height, at)
    }

    /// Stops a private host-local scrollbar thumb drag.
    pub(in crate::win32) fn end_scrollbar_drag(&mut self) -> bool {
        self.lab.end_scrollbar_drag()
    }

    /// Pages one local scrollbar track without queuing a semantic action.
    pub(in crate::win32) fn page_scrollbar_at(
        &mut self,
        width: f32,
        height: f32,
        at: Point,
    ) -> bool {
        self.lab.page_scrollbar_at(width, height, at)
    }

    /// Queues one current pointer-derived semantic action candidate.
    pub(in crate::win32) fn invoke(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(event) = self.lab.event_at(width, height, at) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Moves focus to whatever focusable item is under a pointer position.
    pub(in crate::win32) fn focus_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        self.lab.focus_at(width, height, at)
    }

    /// Queues one current focused semantic action candidate.
    pub(in crate::win32) fn activate_focused(&mut self, width: f32, height: f32) -> bool {
        let Some(event) = self.lab.focused_event(width, height) else {
            return false;
        };
        self.queue_event(event)
    }

    /// Returns whether the current layout has a hovered action.
    pub(in crate::win32) fn is_hovered(&self) -> bool {
        self.lab.hovered.is_some()
    }

    /// Returns the local native renderer state.
    pub(in crate::win32) const fn lab(&self) -> &UiLab {
        &self.lab
    }

    /// Returns the bounded semantic action route for the current document.
    ///
    /// UI Automation receives only this small immutable route, never this
    /// view, the window registry, or a native handle. An initial session has no
    /// document revision to bind an action to and therefore supplies none.
    pub(in crate::win32) fn accessibility_action_sink(
        &self,
    ) -> Option<anodrel_windows_uia::UiAutomationActionSink> {
        anodrel_windows_uia::UiAutomationActionSink::for_current_session(
            self.revision,
            self.input_mailbox.clone(),
        )
    }

    /// Binds this current session revision to its host-only UIA focus route.
    pub(in crate::win32) fn accessibility_focus_route(
        &self,
    ) -> anodrel_windows_uia::UiAutomationFocusRoute {
        self.lab.accessibility_focus_route(Some(self.revision))
    }

    /// Binds this current session revision to its host-only UIA scroll route.
    pub(in crate::win32) fn accessibility_scroll_route(
        &self,
    ) -> anodrel_windows_uia::UiAutomationScrollRoute {
        self.lab.accessibility_scroll_route(Some(self.revision))
    }

    /// Services one UI Automation focus request on the owning UI thread.
    pub(in crate::win32) fn service_accessibility_focus(
        &mut self,
        width: f32,
        height: f32,
    ) -> Option<AccessibilityFocusResult> {
        self.lab
            .service_accessibility_focus(Some(self.revision), width, height)
    }

    /// Services one UI Automation scroll request on the owning UI thread.
    pub(in crate::win32) fn service_accessibility_scroll(
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
