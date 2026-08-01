//! A host-owned visual and input test for `anodrel-ui`.
//!
//! This module is intentionally a diagnostic surface, not an application UI
//! runtime. It renders a fixed document compiled into the host and reports a
//! clicked action ID back into the same host-owned screen. No UI event opens a
//! process, reads a file, sends a protocol message, or grants a capability.

use anodrel_brand::palette;
use anodrel_canvas::{Canvas, Paint, Point, Rect, point};
use anodrel_ui::{
    Action, Axis, ElementId, Insets, Stack, Text, TextMeasurer, UiActionTone, UiDocument, UiEvent,
    UiFocus, UiLayout, UiNode, UiPoint, UiRect, UiSize, UiSurfaceTone, UiTextTone,
};
use anodrel_ui_document::decode;

use super::text;
use super::text::{Align, TextSpec};

const BASE_WIDTH: f32 = 920.0;
const BASE_HEIGHT: f32 = 660.0;
const WEIGHT_REGULAR: i32 = 400;
const UI_LAB_DOCUMENT_JSON: &str = include_str!("ui_lab_document.json");

/// Host-owned state for the UI Lab view.
#[derive(Clone)]
pub(super) struct UiLab {
    document: UiDocument,
    status_target: Option<ElementId>,
    focus: UiFocus,
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
        self.document = document;
        self.focus = UiFocus::new();
        self.hovered = None;
        self.last_action = None;
    }

    fn from_document_with_status(document: UiDocument, status_target: Option<ElementId>) -> Self {
        Self {
            document,
            status_target,
            focus: UiFocus::new(),
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
        let surface = Surface::new(width, height);
        let layout = self.document.layout(surface.bounds(), &WindowsTextMeasurer);
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
        let surface = Surface::new(width, height);
        let layout = self.document.layout(surface.bounds(), &WindowsTextMeasurer);
        self.focus.activate(&layout)
    }

    fn action_at(&self, width: f32, height: f32, at: Point) -> Option<ElementId> {
        let surface = Surface::new(width, height);
        let event = self
            .document
            .layout(surface.bounds(), &WindowsTextMeasurer)
            .hit_test(surface.to_ui_point(at));
        event.map(|UiEvent::ActionInvoked(id)| id)
    }

    fn move_focus(
        &mut self,
        width: f32,
        height: f32,
        move_focus: fn(&mut UiFocus, &UiLayout) -> Option<ElementId>,
    ) -> bool {
        let surface = Surface::new(width, height);
        let layout = self.document.layout(surface.bounds(), &WindowsTextMeasurer);
        let before = self.focus.focused().cloned();
        let after = move_focus(&mut self.focus, &layout);
        before != after
    }
}

/// Draws the UI Lab into one full Anodrel canvas.
pub(super) fn draw(canvas: &mut Canvas, lab: &UiLab) {
    canvas.clear(palette::BACKDROP);
    let surface = Surface::new(canvas.width() as f32, canvas.height() as f32);
    let layout = lab.document.layout(surface.bounds(), &WindowsTextMeasurer);
    let status = status_text(lab);
    draw_node(
        canvas,
        lab,
        &layout,
        lab.document.root(),
        surface,
        lab.status_target.as_ref(),
        status.as_deref(),
    );
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

fn draw_node(
    canvas: &mut Canvas,
    lab: &UiLab,
    layout: &UiLayout,
    node: &UiNode,
    surface: Surface,
    status_target: Option<&ElementId>,
    status: Option<&str>,
) {
    let Some(bounds) = layout.bounds(node.id()) else {
        return;
    };
    match node {
        UiNode::Stack(stack) => {
            if stack.surface_tone() == UiSurfaceTone::Raised {
                let bounds = surface.to_canvas_rect(bounds);
                canvas.fill_rounded_rect(
                    bounds,
                    16.0 * surface.scale,
                    &Paint::solid(palette::PANEL),
                );
                canvas.stroke_rounded_rect(
                    bounds,
                    16.0 * surface.scale,
                    1.0 * surface.scale,
                    &Paint::solid(palette::PANEL_EDGE),
                );
            }
            for child in stack.children() {
                draw_node(canvas, lab, layout, child, surface, status_target, status);
            }
        }
        UiNode::Scroll(scroll) => {
            draw_node(
                canvas,
                lab,
                layout,
                scroll.child(),
                surface,
                status_target,
                status,
            );
        }
        UiNode::Text(text_node) => {
            draw_text(canvas, text_node, bounds, surface, status_target, status);
        }
        UiNode::Action(action) => draw_action(canvas, lab, action, bounds, surface),
    }
}

fn draw_text(
    canvas: &mut Canvas,
    text_node: &Text,
    bounds: UiRect,
    surface: Surface,
    status_target: Option<&ElementId>,
    status: Option<&str>,
) {
    let value = if status_target.is_some_and(|target| target == text_node.id()) {
        status.unwrap_or(text_node.value())
    } else {
        text_node.value()
    };
    let color = match text_node.tone() {
        UiTextTone::Primary => palette::INK,
        UiTextTone::Secondary => palette::INK_SOFT,
        UiTextTone::Accent => palette::ACCENT_SHELL,
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
) {
    let bounds = surface.to_canvas_rect(bounds);
    let hovered = lab.hovered.as_ref() == Some(action.id());
    let focused = lab.focus.focused() == Some(action.id());
    let fill = match (action.tone(), hovered) {
        (UiActionTone::Accent, true) => palette::ACCENT_CORE,
        (UiActionTone::Accent, false) => palette::INDIGO,
        (UiActionTone::Neutral, true) => palette::PANEL_RAISED,
        (UiActionTone::Neutral, false) => palette::BACKDROP_LIFT,
    };
    let edge = if focused {
        palette::ACCENT_IPC
    } else if action.tone() == UiActionTone::Accent || hovered {
        palette::ACCENT_SHELL
    } else {
        palette::PANEL_EDGE
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
        &Paint::solid(palette::INK),
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
    decode(UI_LAB_DOCUMENT_JSON).expect("compiled UI Lab document matches the v1 contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anodrel_ui::UiAccessibilityRole;

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("fixed UI Lab ID is valid")
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
    fn keyboard_focus_traverses_and_activates_only_semantic_actions() {
        let mut lab = UiLab::new();
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
            buttons,
            vec![
                ("ui.lab.inspect", Some("Inspect layout"), true),
                ("ui.lab.hit-test", Some("Test semantic action"), true),
                ("ui.lab.report", Some("Report semantic action"), true),
            ]
        );
    }

    #[test]
    fn visual_hierarchy_comes_from_semantic_roles_not_element_names() {
        let lab = UiLab::new();
        let UiNode::Stack(root) = lab.document.root() else {
            panic!("fixed UI Lab root is a stack");
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
        let emphasized_action = match &actions.children()[1] {
            UiNode::Action(action) => action,
            _ => panic!("fixed UI Lab action is semantic action"),
        };

        assert_eq!(eyebrow.tone(), UiTextTone::Accent);
        assert_eq!(detail.tone(), UiTextTone::Secondary);
        assert_eq!(actions.surface_tone(), UiSurfaceTone::Raised);
        assert_eq!(emphasized_action.tone(), UiActionTone::Accent);
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
