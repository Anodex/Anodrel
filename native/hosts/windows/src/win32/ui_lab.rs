//! A host-owned visual and input test for `anodrel-ui`.
//!
//! This module is intentionally a diagnostic surface, not an application UI
//! runtime. It renders a fixed document compiled into the host and reports a
//! clicked action ID back into the same host-owned screen. No UI event opens a
//! process, reads a file, sends a protocol message, or grants a capability.

use anodrel_brand::palette;
use anodrel_canvas::{Canvas, Color, Paint, Point, Rect, point};
use anodrel_ui::{
    Action, Axis, ElementId, FIELD_HORIZONTAL_PADDING, Field, Insets, Scroll, Stack, Text,
    TextMeasurer, UiActionTone, UiDocument, UiEvent, UiFieldState, UiFieldStates, UiFocus,
    UiLayout, UiLayoutKind, UiNode, UiPoint, UiRect, UiScrollOffsets, UiScrollWheel, UiSize,
    UiSurfaceTone, UiTextTone,
};
use anodrel_ui_document::decode;
use anodrel_ui_session::UiFieldSnapshot;
use anodrel_windows_appearance::{Rgb, SystemAppearance, SystemColors};

use super::text;
use super::text::{Align, TextSpec};

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
        UiNode::Field(_) | UiNode::Text(_) | UiNode::Action(_) => None,
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
    /// What a person has typed into this view's fields.
    ///
    /// Host-owned and never sent anywhere. A document seeds it; after that only
    /// a person changes it. See `docs/UI_FIELDS.md`.
    fields: UiFieldStates,
    pub(super) hovered: Option<ElementId>,
    pub(super) last_action: Option<ElementId>,
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
            item.id() == focused && item.kind() == UiLayoutKind::Field && item.enabled()
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
            fields,
            hovered: None,
            last_action: None,
        }
    }

    /// Updates hover state from one Windows client-area pointer position.
    pub(super) fn update_hover(&mut self, width: f32, height: f32, at: Point) -> bool {
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
        let Some(metrics) = self.layout(width, height).scroll_metrics().first().cloned() else {
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
        let Some(metrics) = self.layout(width, height).scroll_metrics().first().cloned() else {
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

    /// Derives the accessibility semantics for this view's current layout.
    ///
    /// The same layout the surface draws produces these, so what a screen
    /// reader is told cannot drift from what is on screen.
    pub(super) fn accessibility_snapshot(
        &self,
        width: f32,
        height: f32,
    ) -> anodrel_ui::UiAccessibilitySnapshot {
        self.document
            .accessibility_snapshot(&self.layout(width, height))
    }

    fn layout(&self, width: f32, height: f32) -> UiLayout {
        let surface = Surface::new(width, height);
        self.document.layout_with_scroll_offsets(
            surface.bounds(),
            &WindowsTextMeasurer,
            &self.scroll_offsets,
        )
    }
}

/// Draws the UI Lab into one full Anodrel canvas.
pub(super) fn draw(canvas: &mut Canvas, lab: &UiLab) {
    let palette = UiLabPalette::current();
    draw_with_palette(canvas, lab, palette);
}

fn draw_with_palette(canvas: &mut Canvas, lab: &UiLab, palette: UiLabPalette) {
    canvas.clear(palette.backdrop);
    let surface = Surface::new(canvas.width() as f32, canvas.height() as f32);
    let layout = lab.layout(canvas.width() as f32, canvas.height() as f32);
    let status = status_text(lab);
    draw_node(
        canvas,
        lab,
        &layout,
        lab.document.root(),
        surface,
        lab.status_target.as_ref(),
        status.as_deref(),
        palette,
    );
}

/// Concrete host colours for one UI-Lab paint pass.
///
/// The portable UI document chooses only semantic roles. Windows chooses this
/// palette, substituting direct system colours only while high contrast is on.
#[derive(Clone, Copy)]
struct UiLabPalette {
    backdrop: Color,
    backdrop_lift: Color,
    panel: Color,
    panel_raised: Color,
    panel_edge: Color,
    ink: Color,
    ink_soft: Color,
    accent_shell: Color,
    accent_core: Color,
    accent_ipc: Color,
    accent_text: Color,
    button_text: Color,
}

impl UiLabPalette {
    fn current() -> Self {
        let appearance = SystemAppearance::current();
        if appearance.high_contrast() {
            return Self::high_contrast(appearance.colors());
        }
        Self {
            backdrop: palette::BACKDROP,
            backdrop_lift: palette::BACKDROP_LIFT,
            panel: palette::PANEL,
            panel_raised: palette::PANEL_RAISED,
            panel_edge: palette::PANEL_EDGE,
            ink: palette::INK,
            ink_soft: palette::INK_SOFT,
            accent_shell: palette::ACCENT_SHELL,
            accent_core: palette::ACCENT_CORE,
            accent_ipc: palette::ACCENT_IPC,
            accent_text: palette::INK,
            button_text: palette::INK,
        }
    }

    fn high_contrast(colors: SystemColors) -> Self {
        let window = color(colors.window);
        let window_text = color(colors.window_text);
        let button_face = color(colors.button_face);
        let button_text = color(colors.button_text);
        let highlight = color(colors.highlight);
        let highlight_text = color(colors.highlight_text);
        Self {
            backdrop: window,
            backdrop_lift: button_face,
            panel: button_face,
            panel_raised: button_face,
            panel_edge: window_text,
            ink: window_text,
            ink_soft: window_text,
            accent_shell: highlight,
            accent_core: highlight,
            accent_ipc: highlight_text,
            accent_text: highlight_text,
            button_text,
        }
    }
}

const fn color(value: Rgb) -> Color {
    Color::rgb(value.red, value.green, value.blue)
}

/// Base-space layout and its scale into a real client area.
#[derive(Clone, Copy)]
struct Surface {
    scale: f32,
    width: f32,
    height: f32,
}

impl Surface {
    fn new(width: f32, height: f32) -> Self {
        let scale = (width / BASE_WIDTH)
            .min(height / BASE_HEIGHT)
            .clamp(0.6, 4.0);
        Self {
            scale,
            width,
            height,
        }
    }

    fn bounds(self) -> UiRect {
        UiRect::from_size(0.0, 0.0, self.width / self.scale, self.height / self.scale)
    }

    fn to_canvas_rect(self, bounds: UiRect) -> Rect {
        Rect::new(
            bounds.left * self.scale,
            bounds.top * self.scale,
            bounds.right * self.scale,
            bounds.bottom * self.scale,
        )
    }

    fn to_ui_point(self, at: Point) -> UiPoint {
        UiPoint::new(at.x / self.scale, at.y / self.scale)
    }

    fn font(self, size: u16) -> i32 {
        (f32::from(size) * self.scale).round().max(1.0) as i32
    }
}

/// The Windows seam required by the portable UI layout contract.
struct WindowsTextMeasurer;

impl TextMeasurer for WindowsTextMeasurer {
    fn measure(&self, value: &str, font_size: u16) -> UiSize {
        let spec = TextSpec::new(value, i32::from(font_size), WEIGHT_REGULAR);
        UiSize::new(text::width(&spec), text::line_height(&spec))
    }
}

// Recursive rendering takes the exact immutable state for one paint pass.
// Keeping those dependencies visible prevents a process-global render context.
#[allow(clippy::too_many_arguments)]
fn draw_node(
    canvas: &mut Canvas,
    lab: &UiLab,
    layout: &UiLayout,
    node: &UiNode,
    surface: Surface,
    status_target: Option<&ElementId>,
    status: Option<&str>,
    palette: UiLabPalette,
) {
    let Some(item) = layout.items().iter().find(|item| item.id() == node.id()) else {
        return;
    };
    let bounds = item.paint_bounds();
    match node {
        UiNode::Stack(stack) => {
            if stack.surface_tone() == UiSurfaceTone::Raised {
                let bounds = surface.to_canvas_rect(bounds);
                canvas.fill_rounded_rect(
                    bounds,
                    16.0 * surface.scale,
                    &Paint::solid(palette.panel),
                );
                canvas.stroke_rounded_rect(
                    bounds,
                    16.0 * surface.scale,
                    1.0 * surface.scale,
                    &Paint::solid(palette.panel_edge),
                );
            }
            for child in stack.children() {
                draw_node(
                    canvas,
                    lab,
                    layout,
                    child,
                    surface,
                    status_target,
                    status,
                    palette,
                );
            }
        }
        UiNode::Scroll(scroll) => {
            let mut content = Canvas::new(canvas.width(), canvas.height());
            draw_node(
                &mut content,
                lab,
                layout,
                scroll.child(),
                surface,
                status_target,
                status,
                palette,
            );
            canvas.draw_canvas_clipped(&content, 0, 0, 1.0, surface.to_canvas_rect(item.bounds()));
        }
        UiNode::Text(text_node) => {
            draw_text(
                canvas,
                text_node,
                bounds,
                surface,
                status_target,
                status,
                palette,
            );
        }
        UiNode::Action(action) => draw_action(canvas, lab, action, bounds, surface, palette),
        UiNode::Field(field) => draw_field(canvas, lab, field, bounds, surface, palette),
    }
}

/// Draws one field's box, its current host-owned text, and the caret.
///
/// The text comes from the host's field state, not from the document: the
/// document only ever seeded it. See `docs/UI_FIELDS.md`.
fn draw_field(
    canvas: &mut Canvas,
    lab: &UiLab,
    field: &Field,
    bounds: UiRect,
    surface: Surface,
    palette: UiLabPalette,
) {
    let box_bounds = surface.to_canvas_rect(bounds);
    let radius = 8.0 * surface.scale;
    let focused = lab.focus.focused() == Some(field.id());
    canvas.fill_rounded_rect(box_bounds, radius, &Paint::solid(palette.backdrop_lift));
    canvas.stroke_rounded_rect(
        box_bounds,
        radius,
        if focused { 2.0 } else { 1.0 } * surface.scale,
        &Paint::solid(if focused {
            palette.accent_ipc
        } else {
            palette.panel_edge
        }),
    );

    let state = lab.fields.get(field.id());
    let entered = state.map_or("", UiFieldState::text);
    // The placeholder stands in only while there is nothing to show, and is
    // drawn in the dimmer ink so it never reads as entered text.
    let (value, color) = if entered.is_empty() {
        (
            field.placeholder().unwrap_or(""),
            if field.enabled() {
                palette.ink_soft
            } else {
                palette.panel_edge
            },
        )
    } else {
        (
            entered,
            if field.enabled() {
                palette.ink
            } else {
                palette.ink_soft
            },
        )
    };

    let inset = FIELD_HORIZONTAL_PADDING * surface.scale;
    let left = box_bounds.left + inset;
    let font = surface.font(field.font_size());
    let spec = TextSpec::new(value, font, WEIGHT_REGULAR);
    let baseline = box_bounds.top + (box_bounds.height() - text::line_height(&spec)) / 2.0;
    if !value.is_empty() {
        text::draw(
            canvas,
            &spec,
            point(left, baseline),
            Align::Left,
            &Paint::solid(color),
        );
    }

    if !focused {
        return;
    }
    // The caret is placed by measuring the text before it, so it lands between
    // characters at any font and never inside one.
    let Some(state) = state else {
        return;
    };
    let before = TextSpec::new(&entered[..state.caret()], font, WEIGHT_REGULAR);
    let caret_x = left + text::width(&before);
    let caret_height = text::line_height(&spec);
    canvas.fill_rect(
        Rect::new(
            caret_x,
            baseline,
            caret_x + (1.0 * surface.scale).max(1.0),
            baseline + caret_height,
        ),
        &Paint::solid(palette.ink),
    );
}

fn draw_text(
    canvas: &mut Canvas,
    text_node: &Text,
    bounds: UiRect,
    surface: Surface,
    status_target: Option<&ElementId>,
    status: Option<&str>,
    palette: UiLabPalette,
) {
    let value = if status_target.is_some_and(|target| target == text_node.id()) {
        status.unwrap_or(text_node.value())
    } else {
        text_node.value()
    };
    let color = match text_node.tone() {
        UiTextTone::Primary => palette.ink,
        UiTextTone::Secondary => palette.ink_soft,
        UiTextTone::Accent => palette.accent_shell,
    };
    let position = point(bounds.left * surface.scale, bounds.top * surface.scale);
    text::draw(
        canvas,
        &TextSpec::new(value, surface.font(text_node.font_size()), WEIGHT_REGULAR),
        position,
        Align::Left,
        &Paint::solid(color),
    );
}

fn draw_action(
    canvas: &mut Canvas,
    lab: &UiLab,
    action: &Action,
    bounds: UiRect,
    surface: Surface,
    palette: UiLabPalette,
) {
    let bounds = surface.to_canvas_rect(bounds);
    let hovered = lab.hovered.as_ref() == Some(action.id());
    let focused = lab.focus.focused() == Some(action.id());
    let fill = match (action.tone(), hovered) {
        (UiActionTone::Accent, true) => palette.accent_core,
        (UiActionTone::Accent, false) => palette.accent_shell,
        (UiActionTone::Neutral, true) => palette.panel_raised,
        (UiActionTone::Neutral, false) => palette.backdrop_lift,
    };
    let edge = if focused {
        palette.accent_ipc
    } else if action.tone() == UiActionTone::Accent || hovered {
        palette.accent_shell
    } else {
        palette.panel_edge
    };
    canvas.fill_rounded_rect(bounds, 10.0 * surface.scale, &Paint::solid(fill));
    canvas.stroke_rounded_rect(
        bounds,
        10.0 * surface.scale,
        if focused { 2.0 } else { 1.0 } * surface.scale,
        &Paint::solid(edge),
    );

    let spec = TextSpec::new(
        action.label(),
        surface.font(action.font_size()),
        WEIGHT_REGULAR,
    );
    let baseline = bounds.top + (bounds.height() - text::line_height(&spec)) / 2.0;
    text::draw(
        canvas,
        &spec,
        point((bounds.left + bounds.right) / 2.0, baseline),
        Align::Center,
        &Paint::solid(if action.tone() == UiActionTone::Accent {
            palette.accent_text
        } else {
            palette.button_text
        }),
    );
}

fn status_text(lab: &UiLab) -> Option<String> {
    lab.status_target.as_ref()?;
    Some(lab.last_action.as_ref().map_or_else(
        || "Latest semantic event: none".to_owned(),
        |id| format!("Latest semantic event: {id} (no native operation)"),
    ))
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
mod tests {
    use super::*;
    use anodrel_ui::UiAccessibilityRole;

    fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
        Rgb { red, green, blue }
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("fixed UI Lab ID is valid")
    }

    /// Tabs until the sample field has focus, then returns the lab.
    fn focused_on_the_field() -> UiLab {
        let mut lab = UiLab::new();
        for _ in 0..8 {
            if lab.focus.focused() == Some(&id("ui.lab.field")) {
                return lab;
            }
            lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
        }
        panic!("focus never reached the sample field");
    }

    #[test]
    fn typing_reaches_the_focused_field_and_nothing_else() {
        let mut lab = focused_on_the_field();
        for character in "Ada".chars() {
            assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, character));
        }
        assert_eq!(
            lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
            Some("Ada")
        );

        // Typing produces no semantic event, so nothing an application could
        // ever read has changed. See Decision 0067.
        assert_eq!(lab.last_action, None);
    }

    #[test]
    fn typing_with_an_action_focused_changes_nothing() {
        let mut lab = UiLab::new();
        lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
        while lab.focus.focused() == Some(&id("ui.lab.field")) {
            lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
        }
        assert!(!lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'x'));
        assert_eq!(
            lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
            Some("")
        );
    }

    #[test]
    fn editing_keys_move_the_caret_and_remove_characters() {
        let mut lab = focused_on_the_field();
        for character in "abc".chars() {
            assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, character));
        }
        assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Home));
        assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Delete));
        assert_eq!(
            lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
            Some("bc")
        );
        assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::End));
        assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Backspace));
        assert_eq!(
            lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
            Some("b")
        );
    }

    #[test]
    fn a_field_that_left_the_document_cannot_still_be_typed_into() {
        // The focused field is resolved against a fresh layout on every
        // keystroke, so a document replacement that removed it takes effect
        // immediately rather than at the next repaint.
        let mut lab = focused_on_the_field();
        assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'a'));

        lab.replace_document(UiLab::waiting_for_session().document);
        assert!(!lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'b'));
        assert!(lab.fields.get(&id("ui.lab.field")).is_none());
    }

    #[test]
    fn every_action_reports_its_own_semantic_id() {
        let lab = UiLab::new();
        let layout = lab.document.layout(
            UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
            &WindowsTextMeasurer,
        );
        for expected in ["ui.lab.inspect", "ui.lab.hit-test", "ui.lab.report"] {
            let id = ElementId::new(expected).expect("fixed ID is valid");
            let bounds = layout.bounds(&id).expect("action is visible");
            assert_eq!(
                lab.action_at(
                    BASE_WIDTH,
                    BASE_HEIGHT,
                    Point {
                        x: (bounds.left + bounds.right) / 2.0,
                        y: (bounds.top + bounds.bottom) / 2.0,
                    },
                )
                .as_ref()
                .map(ElementId::as_str),
                Some(expected)
            );
        }
    }

    #[test]
    fn high_contrast_palette_uses_only_host_supplied_system_colours() {
        let palette = UiLabPalette::high_contrast(SystemColors {
            window: rgb(1, 2, 3),
            window_text: rgb(4, 5, 6),
            button_face: rgb(7, 8, 9),
            button_text: rgb(10, 11, 12),
            highlight: rgb(13, 14, 15),
            highlight_text: rgb(16, 17, 18),
        });
        assert_eq!(palette.backdrop, Color::rgb(1, 2, 3));
        assert_eq!(palette.panel, Color::rgb(7, 8, 9));
        assert_eq!(palette.ink, Color::rgb(4, 5, 6));
        assert_eq!(palette.accent_shell, Color::rgb(13, 14, 15));
        assert_eq!(palette.accent_text, Color::rgb(16, 17, 18));
        assert_eq!(palette.button_text, Color::rgb(10, 11, 12));
    }

    #[test]
    fn hit_testing_tracks_the_scaled_layout() {
        let lab = UiLab::new();
        let surface = Surface::new(BASE_WIDTH * 2.0, BASE_HEIGHT * 2.0);
        let layout = lab.document.layout(surface.bounds(), &WindowsTextMeasurer);
        let id = ElementId::new("ui.lab.hit-test").expect("fixed ID is valid");
        let bounds = layout.bounds(&id).expect("action is visible");
        assert_eq!(
            lab.action_at(
                BASE_WIDTH * 2.0,
                BASE_HEIGHT * 2.0,
                Point {
                    x: (bounds.left + bounds.right) * surface.scale / 2.0,
                    y: (bounds.top + bounds.bottom) * surface.scale / 2.0,
                },
            ),
            Some(id)
        );
    }

    #[test]
    fn invocation_changes_only_the_host_owned_status() {
        let mut lab = UiLab::new();
        let layout = lab.document.layout(
            UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
            &WindowsTextMeasurer,
        );
        let id = ElementId::new("ui.lab.inspect").expect("fixed ID is valid");
        let bounds = layout.bounds(&id).expect("action is visible");
        assert!(lab.invoke(
            BASE_WIDTH,
            BASE_HEIGHT,
            Point {
                x: (bounds.left + bounds.right) / 2.0,
                y: (bounds.top + bounds.bottom) / 2.0,
            },
        ));
        assert_eq!(lab.last_action, Some(id));
    }

    #[test]
    fn keyboard_focus_traverses_fields_and_actions_but_activates_only_actions() {
        // Renamed from "traverses and activates only semantic actions": Tab now
        // reaches a field too, because a person has to get to one to type. What
        // has not changed is that only an action can be activated.
        let mut lab = UiLab::new();
        assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
        assert_eq!(lab.focus.focused(), Some(&id("ui.lab.field")));
        assert!(
            !lab.activate_focused(BASE_WIDTH, BASE_HEIGHT),
            "a focused field was activated"
        );
        assert_eq!(lab.last_action, None);

        assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
        assert_eq!(lab.focus.focused(), Some(&id("ui.lab.inspect")));
        assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
        assert_eq!(lab.focus.focused(), Some(&id("ui.lab.hit-test")));
        assert!(lab.activate_focused(BASE_WIDTH, BASE_HEIGHT));
        assert_eq!(lab.last_action, Some(id("ui.lab.hit-test")));
        assert!(lab.focus_previous(BASE_WIDTH, BASE_HEIGHT));
        assert_eq!(lab.focus.focused(), Some(&id("ui.lab.inspect")));
    }

    #[test]
    fn exposes_the_same_button_semantics_that_the_lab_draws() {
        let lab = UiLab::new();
        let layout = lab.document.layout(
            UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
            &WindowsTextMeasurer,
        );
        let snapshot = lab.document.accessibility_snapshot(&layout);
        let buttons = snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == UiAccessibilityRole::Button)
            .map(|node| (node.id().as_str(), node.name(), node.enabled()))
            .collect::<Vec<_>>();

        assert_eq!(
            &buttons[..3],
            [
                ("ui.lab.inspect", Some("Inspect layout"), true),
                ("ui.lab.hit-test", Some("Test semantic action"), true),
                ("ui.lab.report", Some("Report semantic action"), true),
            ]
        );
    }

    #[test]
    fn visual_hierarchy_comes_from_semantic_roles_not_element_names() {
        let lab = UiLab::new();
        let UiNode::Scroll(viewport) = lab.document.root() else {
            panic!("fixed UI Lab root is a scroll viewport");
        };
        let UiNode::Stack(content) = viewport.child() else {
            panic!("fixed UI Lab viewport has a content stack");
        };
        let UiNode::Stack(root) = &content.children()[0] else {
            panic!("fixed UI Lab fixture is a stack");
        };

        let eyebrow = match &root.children()[0] {
            UiNode::Text(text) => text,
            _ => panic!("fixed UI Lab eyebrow is text"),
        };
        let detail = match &root.children()[2] {
            UiNode::Text(text) => text,
            _ => panic!("fixed UI Lab detail is text"),
        };
        let UiNode::Stack(actions) = &root.children()[3] else {
            panic!("fixed UI Lab actions are a stack");
        };
        // Found by ID rather than by position: this document gains nodes over
        // time, and an index would keep silently pointing at a different one.
        let emphasized_action = actions
            .children()
            .iter()
            .find_map(|child| match child {
                UiNode::Action(action) if action.id() == &id("ui.lab.hit-test") => Some(action),
                _ => None,
            })
            .expect("fixed UI Lab emphasized action exists");

        assert_eq!(eyebrow.tone(), UiTextTone::Accent);
        assert_eq!(detail.tone(), UiTextTone::Secondary);
        assert_eq!(actions.surface_tone(), UiSurfaceTone::Raised);
        assert_eq!(emphasized_action.tone(), UiActionTone::Accent);
    }

    #[test]
    fn page_scrolling_changes_only_the_lab_owned_viewport_position() {
        let mut lab = UiLab::new();

        assert!(lab.scroll_page(BASE_WIDTH, BASE_HEIGHT, true));
        assert!(lab.scroll_offsets[&id("ui.lab.viewport")].offset_y() > 0.0);
        assert!(
            lab.layout(BASE_WIDTH, BASE_HEIGHT)
                .bounds(&id("ui.lab.scroll.exercise-9"))
                .is_some()
        );
    }

    #[test]
    fn preview_documents_have_no_lab_specific_status_replacement() {
        let document = decode(
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"External text","fontSize":16,"tone":"primary"}}"#,
        )
        .expect("preview fixture is valid");
        let preview = UiLab::preview(document);

        assert!(preview.status_target.is_none());
        assert_eq!(status_text(&preview), None);
    }

    #[test]
    fn preview_document_renders_through_the_same_native_ui_view() {
        let document = decode(
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":40,"top":40,"right":40,"bottom":40},"gap":12,"surfaceTone":"plain","children":[{"id":"title","kind":"text","value":"External preview document","fontSize":28,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#,
        )
        .expect("preview fixture is valid");
        let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
        draw(&mut canvas, &UiLab::preview(document));

        let changed = (0..canvas.height())
            .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
            .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
            .count();
        assert!(changed > 1_000, "preview drew too little content");
    }

    #[test]
    fn draws_visible_content_without_a_web_surface() {
        let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
        draw(&mut canvas, &UiLab::new());
        let changed = (0..canvas.height())
            .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
            .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
            .count();
        assert!(changed > 1_000, "UI Lab drew too little content");
    }
}
