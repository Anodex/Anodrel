//! Local document, focus, field, pointer, and keyboard state for the UI Lab.

use super::*;

fn find_field<'a>(node: &'a UiNode, id: &ElementId) -> Option<&'a Field> {
    match node {
        UiNode::Field(field) if field.id() == id => Some(field),
        UiNode::Stack(stack) => stack
            .children()
            .iter()
            .find_map(|child| find_field(child, id)),
        UiNode::Scroll(scroll) => find_field(scroll.child(), id),
        UiNode::Field(_) | UiNode::Text(_) | UiNode::Status(_) | UiNode::Action(_) => None,
    }
}

/// Host-owned state for the UI Lab view.
impl UiLab {
    /// Builds the fixed visual test document.
    pub(in crate::win32) fn new() -> Self {
        let status_target = ElementId::new("ui.lab.status").expect("fixed UI Lab ID is valid");
        Self::from_document_with_status(fixture::document(), Some(status_target))
    }

    /// Builds a local diagnostic view around one already-validated document.
    ///
    /// A preview has no host status binding: its nodes display exactly the text
    /// carried by the document, and its action events stay local to this view.
    pub(in crate::win32) fn preview(document: UiDocument) -> Self {
        Self::from_document_with_status(document, None)
    }

    /// Builds a host-owned waiting document for an authenticated session view.
    pub(in crate::win32) fn waiting_for_session() -> Self {
        let root = UiNode::Stack(
            Stack::new(
                ElementId::new("session.waiting.root").expect("fixed waiting ID is valid"),
                Axis::Vertical,
                Insets::new(56, 56, 56, 56).expect("fixed waiting padding is valid"),
                16,
                vec![
                    UiNode::Text(
                        Text::new(
                            ElementId::new("session.waiting.eyebrow")
                                .expect("fixed waiting ID is valid"),
                            "ANODREL UI SESSION",
                            14,
                        )
                        .expect("fixed waiting text is valid")
                        .with_tone(UiTextTone::Accent),
                    ),
                    UiNode::Text(
                        Text::new(
                            ElementId::new("session.waiting.title")
                                .expect("fixed waiting ID is valid"),
                            "Waiting for an authenticated document",
                            28,
                        )
                        .expect("fixed waiting text is valid"),
                    ),
                    UiNode::Text(
                        Text::new(
                            ElementId::new("session.waiting.detail")
                                .expect("fixed waiting ID is valid"),
                            "The native host will apply only the latest accepted session revision.",
                            16,
                        )
                        .expect("fixed waiting text is valid")
                        .with_tone(UiTextTone::Secondary),
                    ),
                ],
            )
            .expect("fixed waiting stack is valid"),
        );
        Self::from_document_with_status(
            UiDocument::new(root).expect("fixed waiting document is valid"),
            None,
        )
    }

    /// Replaces this local visual document and discards stale local input state.
    pub(in crate::win32) fn replace_document(&mut self, document: UiDocument) {
        // Reseeding discards what was typed. That follows from a document being
        // a whole snapshot rather than a patch; Decision 0067 records it as a
        // consequence an application has to know about.
        self.fields.reseed(&document);
        self.document = document;
        self.focus = UiFocus::new();
        self.scroll_offsets.clear();
        self.wheel.clear();
        self.scrollbar_drag = None;
        self.hovered = None;
        self.last_action = None;
    }

    /// Applies one typed character to the focused field.
    ///
    /// Returns whether anything changed, so the caller repaints only when it
    /// did. A character arriving with no field focused, or one the field
    /// refuses, changes nothing and is silently dropped — this is a person
    /// typing, not an operation that needs to report a failure.
    pub(in crate::win32) fn type_character(
        &mut self,
        width: f32,
        height: f32,
        character: char,
    ) -> bool {
        let Some(field) = self.focused_field(width, height) else {
            return false;
        };
        let Some(state) = self.fields.get_mut(field.id()) else {
            return false;
        };
        state.insert(character, &field)
    }

    /// Applies one editing key to the focused field.
    pub(in crate::win32) fn edit_focused_field(
        &mut self,
        width: f32,
        height: f32,
        edit: FieldEdit,
    ) -> bool {
        let Some(field) = self.focused_field(width, height) else {
            return false;
        };
        let Some(state) = self.fields.get_mut(field.id()) else {
            return false;
        };
        match edit {
            FieldEdit::Backspace => state.backspace(),
            FieldEdit::Delete => state.delete(),
            FieldEdit::Left => state.move_left(),
            FieldEdit::Right => state.move_right(),
            FieldEdit::Home => {
                state.move_home();
                true
            }
            FieldEdit::End => {
                state.move_end();
                true
            }
        }
    }

    /// Returns every field value on this view, for a granted read.
    ///
    /// Built from the host's own state, in element-ID order, carrying values
    /// only. See `docs/UI_FIELDS.md` and Decision 0067.
    pub(in crate::win32) fn field_snapshot(&self) -> Option<UiFieldSnapshot> {
        UiFieldSnapshot::from_states(&self.fields).ok()
    }

    /// Returns the focused field, if focus is on one that is still visible.
    ///
    /// Resolved against a fresh layout every time rather than remembered, so a
    /// field that was removed, clipped, or disabled since the last keystroke
    /// cannot still be typed into.
    pub(in crate::win32) fn focused_field(&self, width: f32, height: f32) -> Option<Field> {
        let focused = self.focus.focused()?;
        let layout = self.layout(width, height);
        layout.items().iter().find(|item| {
            item.id() == focused
                && item.kind() == UiLayoutKind::Field
                && item.enabled()
                && !item.bounds().is_empty()
        })?;
        find_field(self.document.root(), focused).cloned()
    }

    pub(in crate::win32) fn from_document_with_status(
        document: UiDocument,
        status_target: Option<ElementId>,
    ) -> Self {
        let mut fields = UiFieldStates::new();
        fields.reseed(&document);
        Self {
            document,
            status_target,
            focus: UiFocus::new(),
            scroll_offsets: UiScrollOffsets::new(),
            wheel: UiScrollWheel::default(),
            scrollbar_drag: None,
            scrollbar_release_pending: false,
            fields,
            automation_focus: UiAutomationFocusMailbox::new(),
            automation_scroll: UiAutomationScrollMailbox::new(),
            hovered: None,
            last_action: None,
        }
    }

    /// Updates hover state from one Windows client-area pointer position.
    pub(in crate::win32) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
        if self.scrollbar_drag.is_some() {
            return false;
        }
        let hovered = self.action_at(width, height, at);
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }

    /// Clears the hover state, returning whether a repaint is needed.
    pub(in crate::win32) fn clear_hover(&mut self) -> bool {
        let changed = self.hovered.is_some();
        self.hovered = None;
        changed
    }

    /// Records a semantic action event and returns whether the view changed.
    pub(in crate::win32) fn invoke(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(action) = self.action_at(width, height, at) else {
            return false;
        };
        let changed = self.last_action.as_ref() != Some(&action);
        self.last_action = Some(action);
        changed
    }

    /// Moves focus to whatever focusable item is under a pointer position.
    ///
    /// Separate from [`invoke`](Self::invoke) because the two answer different
    /// questions: this one is "what should now have focus", and a field can be
    /// the answer to it while never being the answer to "what did this
    /// activate". Returns whether focus changed, so a caller repaints only when
    /// the ring actually moved.
    pub(in crate::win32) fn focus_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        let surface = Surface::new(width, height);
        let layout = self.layout(width, height);
        let Some(target) = layout.focus_target_at(surface.to_ui_point(at)).cloned() else {
            return false;
        };
        self.focus.focus_on(&layout, &target)
    }

    /// Moves focus forward through this view's current visible action layout.
    pub(in crate::win32) fn focus_next(&mut self, width: f32, height: f32) -> bool {
        self.move_focus(width, height, UiFocus::move_next)
    }

    /// Moves focus backward through this view's current visible action layout.
    pub(in crate::win32) fn focus_previous(&mut self, width: f32, height: f32) -> bool {
        self.move_focus(width, height, UiFocus::move_previous)
    }

    /// Records the semantic action associated with the current valid focus.
    pub(in crate::win32) fn activate_focused(&mut self, width: f32, height: f32) -> bool {
        let layout = self.layout(width, height);
        let Some(UiEvent::ActionInvoked(action)) = self.focus.activate(&layout) else {
            return false;
        };
        let changed = self.last_action.as_ref() != Some(&action);
        self.last_action = Some(action);
        changed
    }

    /// Returns the semantic pointer event at one current layout position.
    pub(in crate::win32) fn event_at(&self, width: f32, height: f32, at: Point) -> Option<UiEvent> {
        self.action_at(width, height, at)
            .map(UiEvent::ActionInvoked)
    }

    /// Returns the current focused semantic event without recording local
    /// diagnostic action state.
    pub(in crate::win32) fn focused_event(&mut self, width: f32, height: f32) -> Option<UiEvent> {
        let layout = self.layout(width, height);
        self.focus.activate(&layout)
    }

    pub(in crate::win32) fn action_at(
        &self,
        width: f32,
        height: f32,
        at: Point,
    ) -> Option<ElementId> {
        let surface = Surface::new(width, height);
        let event = self.layout(width, height).hit_test(surface.to_ui_point(at));
        event.map(|UiEvent::ActionInvoked(id)| id)
    }

    pub(in crate::win32) fn move_focus(
        &mut self,
        width: f32,
        height: f32,
        move_focus: fn(&mut UiFocus, &UiLayout) -> Option<ElementId>,
    ) -> bool {
        let layout = self.layout(width, height);
        let before = self.focus.focused().cloned();
        let after = move_focus(&mut self.focus, &layout);
        before != after
    }
}
