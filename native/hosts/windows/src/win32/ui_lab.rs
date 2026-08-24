//! A host-owned visual and input test for `anodrel-ui`.
//!
//! This module is intentionally a diagnostic surface, not an application UI
//! runtime. It renders a fixed document compiled into the host and reports a
//! clicked action ID back into the same host-owned screen. No UI event opens a
//! process, reads a file, sends a protocol message, or grants a capability.

mod accessibility;
mod render;

use anodrel_brand::palette;
use anodrel_canvas::{Canvas, Color, Paint, Point, Rect, point};
use anodrel_ui::{
    Action, Axis, ElementId, FIELD_HORIZONTAL_PADDING, Field, Insets, Scroll, Stack, Text,
    TextMeasurer, UiActionTone, UiDocument, UiEvent, UiFieldState, UiFieldStates, UiFocus,
    UiLayout, UiLayoutKind, UiNode, UiPoint, UiRect, UiScrollOffsets, UiScrollState, UiScrollWheel,
    UiSize, UiSurfaceTone, UiTextTone, wrap_text,
};
use anodrel_ui_document::decode;
use anodrel_ui_session::UiFieldSnapshot;
use anodrel_windows_appearance::{Rgb, SystemAppearance, SystemColors};
use anodrel_windows_uia::{
    UiAutomationFocusMailbox, UiAutomationFocusRoute, UiAutomationScrollCommand,
    UiAutomationScrollMailbox, UiAutomationScrollRoute, UiAutomationScrollSnapshot,
};

use super::scrollbar::{Scrollbar, ScrollbarHit};
use super::text;
use super::text::{Align, TextSpec};
use render::{Surface, WindowsTextMeasurer};
#[cfg(test)]
use render::{UiLabPalette, status_text};

/// Draws the UI Lab through its dedicated software-canvas renderer.
pub(super) fn draw(canvas: &mut Canvas, lab: &UiLab) {
    render::draw(canvas, lab);
}

const BASE_WIDTH: f32 = 920.0;
const BASE_HEIGHT: f32 = 660.0;
const WEIGHT_REGULAR: i32 = 400;
const UI_LAB_DOCUMENT_JSON: &str = include_str!("ui_lab_document.json");

/// One editing key applied to a focused field.
///
/// A closed set. Every key the host forwards is named here, so a future key
/// cannot arrive as an opaque code the field logic has to interpret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FieldEdit {
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

/// Returns the field with one element ID, if the document holds it.
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
#[derive(Clone)]
pub(super) struct UiLab {
    document: UiDocument,
    status_target: Option<ElementId>,
    focus: UiFocus,
    scroll_offsets: UiScrollOffsets,
    wheel: UiScrollWheel,
    scrollbar_drag: Option<ScrollbarDrag>,
    /// A thumb press remains a scrollbar gesture until its matching release,
    /// even if a newer document removes the viewport while it is captured.
    scrollbar_release_pending: bool,
    /// What a person has typed into this view's fields.
    ///
    /// Host-owned and never sent anywhere. A document seeds it; after that only
    /// a person changes it. See `docs/UI_FIELDS.md`.
    fields: UiFieldStates,
    /// The private UI Automation route for this one native view.
    ///
    /// A provider receives only a revision-bound route, never this mutable
    /// view. The UI thread revalidates every request below.
    automation_focus: UiAutomationFocusMailbox,
    /// The private UI Automation route for this view's retained scroll state.
    automation_scroll: UiAutomationScrollMailbox,
    pub(super) hovered: Option<ElementId>,
    pub(super) last_action: Option<ElementId>,
}

/// Private state retained only while Windows has captured one scrollbar thumb.
#[derive(Clone)]
struct ScrollbarDrag {
    id: ElementId,
    grab_offset_y: f32,
}

/// The local result of one UI Automation focus request.
///
/// `accepted` stays true when the requested element was already focused: that
/// is a truthful `SetFocus` success, but it is not a new focus transition to
/// announce to Windows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AccessibilityFocusResult {
    pub(super) accepted: bool,
    pub(super) changed: bool,
}

/// The local result of one UI Automation scroll request.
///
/// An accepted request can already be at its requested position. That remains
/// a truthful UI Automation success without a repaint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AccessibilityScrollResult {
    pub(super) accepted: bool,
    pub(super) changed: bool,
}

impl UiLab {
    /// Builds the fixed visual test document.
    pub(super) fn new() -> Self {
        let status_target = ElementId::new("ui.lab.status").expect("fixed UI Lab ID is valid");
        Self::from_document_with_status(test_document(), Some(status_target))
    }

    /// Builds a local diagnostic view around one already-validated document.
    ///
    /// A preview has no host status binding: its nodes display exactly the text
    /// carried by the document, and its action events stay local to this view.
    pub(super) fn preview(document: UiDocument) -> Self {
        Self::from_document_with_status(document, None)
    }

    /// Builds a host-owned waiting document for an authenticated session view.
    pub(super) fn waiting_for_session() -> Self {
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
    pub(super) fn replace_document(&mut self, document: UiDocument) {
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
    pub(super) fn type_character(&mut self, width: f32, height: f32, character: char) -> bool {
        let Some(field) = self.focused_field(width, height) else {
            return false;
        };
        let Some(state) = self.fields.get_mut(field.id()) else {
            return false;
        };
        state.insert(character, &field)
    }

    /// Applies one editing key to the focused field.
    pub(super) fn edit_focused_field(&mut self, width: f32, height: f32, edit: FieldEdit) -> bool {
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
    pub(super) fn field_snapshot(&self) -> Option<UiFieldSnapshot> {
        UiFieldSnapshot::from_states(&self.fields).ok()
    }

    /// Returns the focused field, if focus is on one that is still visible.
    ///
    /// Resolved against a fresh layout every time rather than remembered, so a
    /// field that was removed, clipped, or disabled since the last keystroke
    /// cannot still be typed into.
    fn focused_field(&self, width: f32, height: f32) -> Option<Field> {
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

    fn from_document_with_status(document: UiDocument, status_target: Option<ElementId>) -> Self {
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
    pub(super) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
        if self.scrollbar_drag.is_some() {
            return false;
        }
        let hovered = self.action_at(width, height, at);
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }

    /// Clears the hover state, returning whether a repaint is needed.
    pub(super) fn clear_hover(&mut self) -> bool {
        let changed = self.hovered.is_some();
        self.hovered = None;
        changed
    }

    /// Records a semantic action event and returns whether the view changed.
    pub(super) fn invoke(&mut self, width: f32, height: f32, at: Point) -> bool {
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
    pub(super) fn focus_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        let surface = Surface::new(width, height);
        let layout = self.layout(width, height);
        let Some(target) = layout.focus_target_at(surface.to_ui_point(at)).cloned() else {
            return false;
        };
        self.focus.focus_on(&layout, &target)
    }

    /// Moves focus forward through this view's current visible action layout.
    pub(super) fn focus_next(&mut self, width: f32, height: f32) -> bool {
        self.move_focus(width, height, UiFocus::move_next)
    }

    /// Moves focus backward through this view's current visible action layout.
    pub(super) fn focus_previous(&mut self, width: f32, height: f32) -> bool {
        self.move_focus(width, height, UiFocus::move_previous)
    }

    /// Records the semantic action associated with the current valid focus.
    pub(super) fn activate_focused(&mut self, width: f32, height: f32) -> bool {
        let layout = self.layout(width, height);
        let Some(UiEvent::ActionInvoked(action)) = self.focus.activate(&layout) else {
            return false;
        };
        let changed = self.last_action.as_ref() != Some(&action);
        self.last_action = Some(action);
        changed
    }

    /// Returns the semantic pointer event at one current layout position.
    pub(super) fn event_at(&self, width: f32, height: f32, at: Point) -> Option<UiEvent> {
        self.action_at(width, height, at)
            .map(UiEvent::ActionInvoked)
    }

    /// Returns the current focused semantic event without recording local
    /// diagnostic action state.
    pub(super) fn focused_event(&mut self, width: f32, height: f32) -> Option<UiEvent> {
        let layout = self.layout(width, height);
        self.focus.activate(&layout)
    }

    fn action_at(&self, width: f32, height: f32, at: Point) -> Option<ElementId> {
        let surface = Surface::new(width, height);
        let event = self.layout(width, height).hit_test(surface.to_ui_point(at));
        event.map(|UiEvent::ActionInvoked(id)| id)
    }

    fn move_focus(
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

    /// Moves the first visible diagnostic scroll viewport by one page.
    ///
    /// This is local Windows Lab behavior only. It does not produce an
    /// application event or carry native authority.
    pub(super) fn scroll_page(&mut self, width: f32, height: f32, forward: bool) -> bool {
        let layout = self.layout(width, height);
        let Some(metrics) = Self::first_overflowing_scroll_metric(&layout) else {
            return false;
        };
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_page(forward, metrics.viewport_height(), metrics.content_height());
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Moves the first visible diagnostic scroll viewport by one local line.
    pub(super) fn scroll_line(&mut self, width: f32, height: f32, forward: bool) -> bool {
        let layout = self.layout(width, height);
        let Some(metrics) = Self::first_overflowing_scroll_metric(&layout) else {
            return false;
        };
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_line(forward, metrics.viewport_height(), metrics.content_height());
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Converts one native wheel delta into owned whole-line movement.
    pub(super) fn scroll_wheel_delta(&mut self, width: f32, height: f32, delta: i32) -> bool {
        let lines = self.wheel.push(delta);
        let forward = lines < 0;
        (0..lines.unsigned_abs()).any(|_| self.scroll_line(width, height, forward))
    }

    /// Begins one host-local scrollbar thumb drag when the pointer is on it.
    ///
    /// The returned state contains no raw pointer data after the caller's
    /// current message. The Win32 owner uses it only to decide whether to
    /// capture the pointer for this native window.
    pub(super) fn begin_scrollbar_drag(&mut self, width: f32, height: f32, at: Point) -> bool {
        let surface = Surface::new(width, height);
        let Some((scrollbar, _)) = self.first_scrollbar(width, height) else {
            return false;
        };
        let Some(ScrollbarHit::Thumb { grab_offset_y }) =
            scrollbar.hit_test(surface.to_ui_point(at))
        else {
            return false;
        };
        self.scrollbar_drag = Some(ScrollbarDrag {
            id: scrollbar.id().clone(),
            grab_offset_y,
        });
        self.scrollbar_release_pending = true;
        self.hovered = None;
        true
    }

    /// Applies one captured pointer position to the retained scrollbar offset.
    pub(super) fn drag_scrollbar(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(drag) = self.scrollbar_drag.clone() else {
            return false;
        };
        let Some((scrollbar, metrics)) = self.first_scrollbar(width, height) else {
            return false;
        };
        if scrollbar.id() != &drag.id {
            return false;
        }
        let surface = Surface::new(width, height);
        let requested =
            scrollbar.offset_for_thumb_grab(surface.to_ui_point(at).y, drag.grab_offset_y);
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_to(
                requested,
                metrics.viewport_height(),
                metrics.content_height(),
            );
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Stops a host-local thumb drag after a release or capture loss.
    pub(super) fn end_scrollbar_drag(&mut self) -> bool {
        let ended = self.scrollbar_drag.take().is_some() || self.scrollbar_release_pending;
        self.scrollbar_release_pending = false;
        ended
    }

    /// Moves one host-owned scrollbar by a page when its track was pressed.
    ///
    /// A thumb hit is also consumed, so an opaque overlay cannot activate an
    /// action that happens to be painted beneath it.
    pub(super) fn page_scrollbar_at(&mut self, width: f32, height: f32, at: Point) -> bool {
        let surface = Surface::new(width, height);
        let Some((scrollbar, metrics)) = self.first_scrollbar(width, height) else {
            return false;
        };
        let Some(hit) = scrollbar.hit_test(surface.to_ui_point(at)) else {
            return false;
        };
        let changed = match hit {
            ScrollbarHit::Thumb { .. } => false,
            ScrollbarHit::TrackBefore => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_page(false, metrics.viewport_height(), metrics.content_height()),
            ScrollbarHit::TrackAfter => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_page(true, metrics.viewport_height(), metrics.content_height()),
        };
        if changed {
            self.hovered = None;
        }
        true
    }

    /// Clamps retained scroll positions after a native size change.
    pub(super) fn clamp_scroll_offsets(&mut self, width: f32, height: f32) {
        let metrics = self.layout(width, height).scroll_metrics().to_vec();
        for metric in metrics {
            self.scroll_offsets
                .entry(metric.id().clone())
                .or_default()
                .clamp(metric.viewport_height(), metric.content_height());
        }
    }

    fn layout(&self, width: f32, height: f32) -> UiLayout {
        let surface = Surface::new(width, height);
        self.document.layout_with_scroll_offsets(
            surface.bounds(),
            &WindowsTextMeasurer,
            &self.scroll_offsets,
        )
    }

    fn first_scrollbar(
        &self,
        width: f32,
        height: f32,
    ) -> Option<(Scrollbar, anodrel_ui::UiScrollMetrics)> {
        let layout = self.layout(width, height);
        self.first_scrollbar_in_layout(&layout)
    }

    fn first_scrollbar_in_layout(
        &self,
        layout: &UiLayout,
    ) -> Option<(Scrollbar, anodrel_ui::UiScrollMetrics)> {
        let metrics = Self::first_overflowing_scroll_metric(layout)?;
        let viewport = layout.bounds(metrics.id())?;
        let offset = self
            .scroll_offsets
            .get(metrics.id())
            .copied()
            .map_or(0.0, |state| state.offset_y());
        Scrollbar::from_metric(&metrics, viewport, offset).map(|scrollbar| (scrollbar, metrics))
    }

    fn first_overflowing_scroll_metric(layout: &UiLayout) -> Option<anodrel_ui::UiScrollMetrics> {
        layout
            .scroll_metrics()
            .iter()
            .find(|metrics| {
                layout.bounds(metrics.id()).is_some()
                    && UiScrollState::maximum_offset(
                        metrics.viewport_height(),
                        metrics.content_height(),
                    ) > 0.0
            })
            .cloned()
    }
}

fn test_document() -> UiDocument {
    let fixture =
        decode(UI_LAB_DOCUMENT_JSON).expect("compiled UI Lab document matches the v1 contract");
    let scroll_exercises = UiNode::Stack(
        Stack::new(
            ElementId::new("ui.lab.scroll.exercises").expect("fixed scroll ID is valid"),
            Axis::Vertical,
            Insets::all(18).expect("fixed scroll padding is valid"),
            10,
            (1..=9)
                .map(|index| {
                    UiNode::Action(
                        Action::new(
                            ElementId::new(format!("ui.lab.scroll.exercise-{index}"))
                                .expect("fixed scroll action ID is valid"),
                            format!("Scroll exercise {index}"),
                            15,
                            true,
                        )
                        .expect("fixed scroll action is valid"),
                    )
                })
                .collect(),
        )
        .expect("fixed scroll stack is valid")
        .with_surface_tone(UiSurfaceTone::Raised),
    );
    UiDocument::new(UiNode::Scroll(Scroll::new(
        ElementId::new("ui.lab.viewport").expect("fixed scroll viewport ID is valid"),
        UiNode::Stack(
            Stack::new(
                ElementId::new("ui.lab.scroll.content").expect("fixed scroll content ID is valid"),
                Axis::Vertical,
                Insets::zero(),
                18,
                vec![fixture.root().clone(), scroll_exercises],
            )
            .expect("fixed scroll content stack is valid"),
        ),
    )))
    .expect("fixed scroll document is valid")
}

#[cfg(test)]
mod tests;
