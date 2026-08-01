//! A host-owned visual and input test for `anodrel-ui`.
//!
//! This module is intentionally a diagnostic surface, not an application UI
//! runtime. It renders a fixed document constructed by the host and reports a
//! clicked action ID back into the same host-owned screen. No UI event opens a
//! process, reads a file, sends a protocol message, or grants a capability.

use anodrel_brand::palette;
use anodrel_canvas::{Canvas, Paint, Point, Rect, point};
use anodrel_ui::{
    Action, Axis, ElementId, Insets, Stack, Text, TextMeasurer, UiDocument, UiEvent, UiLayout,
    UiNode, UiPoint, UiRect, UiSize,
};

use super::text;
use super::text::{Align, TextSpec};

const BASE_WIDTH: f32 = 920.0;
const BASE_HEIGHT: f32 = 660.0;
const WEIGHT_REGULAR: i32 = 400;

/// Host-owned state for the UI Lab view.
#[derive(Clone)]
pub(super) struct UiLab {
    document: UiDocument,
    pub(super) hovered: Option<ElementId>,
    pub(super) last_action: Option<ElementId>,
}

impl UiLab {
    /// Builds the fixed visual test document.
    pub(super) fn new() -> Self {
        Self {
            document: test_document(),
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

    fn action_at(&self, width: f32, height: f32, at: Point) -> Option<ElementId> {
        let surface = Surface::new(width, height);
        let event = self
            .document
            .layout(surface.bounds(), &WindowsTextMeasurer)
            .hit_test(surface.to_ui_point(at));
        event.map(|UiEvent::ActionInvoked(id)| id)
    }
}

/// Draws the UI Lab into one full Anodrel canvas.
pub(super) fn draw(canvas: &mut Canvas, lab: &UiLab) {
    canvas.clear(palette::BACKDROP);
    let surface = Surface::new(canvas.width() as f32, canvas.height() as f32);
    let layout = lab.document.layout(surface.bounds(), &WindowsTextMeasurer);
    let status = status_text(lab);
    draw_node(canvas, lab, &layout, lab.document.root(), surface, &status);
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
    status: &str,
) {
    let Some(bounds) = layout.bounds(node.id()) else {
        return;
    };
    match node {
        UiNode::Stack(stack) => {
            if stack.id().as_str() == "ui.lab.actions" {
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
                draw_node(canvas, lab, layout, child, surface, status);
            }
        }
        UiNode::Text(text_node) => draw_text(canvas, text_node, bounds, surface, status),
        UiNode::Action(action) => draw_action(canvas, lab, action, bounds, surface),
    }
}

fn draw_text(
    canvas: &mut Canvas,
    text_node: &Text,
    bounds: UiRect,
    surface: Surface,
    status: &str,
) {
    let id = text_node.id().as_str();
    let value = if id == "ui.lab.status" {
        status
    } else {
        text_node.value()
    };
    let color = match id {
        "ui.lab.eyebrow" => palette::ACCENT_SHELL,
        "ui.lab.title" => palette::INK,
        "ui.lab.status" => palette::ACCENT_PACKAGE,
        _ => palette::INK_SOFT,
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
    let fill = if hovered {
        palette::PANEL_RAISED
    } else {
        palette::BACKDROP_LIFT
    };
    let edge = if hovered {
        palette::ACCENT_SHELL
    } else {
        palette::PANEL_EDGE
    };
    canvas.fill_rounded_rect(bounds, 10.0 * surface.scale, &Paint::solid(fill));
    canvas.stroke_rounded_rect(
        bounds,
        10.0 * surface.scale,
        1.0 * surface.scale,
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

fn status_text(lab: &UiLab) -> String {
    lab.last_action.as_ref().map_or_else(
        || "Latest semantic event: none".to_owned(),
        |id| format!("Latest semantic event: {id} (no native operation)"),
    )
}

fn test_document() -> UiDocument {
    let actions = stack(
        "ui.lab.actions",
        Axis::Vertical,
        Insets::new(18, 18, 18, 18).expect("fixed UI Lab padding is valid"),
        10,
        vec![
            action("ui.lab.inspect", "Inspect layout"),
            action("ui.lab.hit-test", "Test semantic action"),
            action("ui.lab.report", "Report semantic action"),
        ],
    );
    UiDocument::new(stack(
        "ui.lab.root",
        Axis::Vertical,
        Insets::new(54, 54, 54, 44).expect("fixed UI Lab padding is valid"),
        14,
        vec![
            text("ui.lab.eyebrow", "OWNED NATIVE UI FOUNDATION", 12),
            text("ui.lab.title", "Anodrel UI Lab", 34),
            text(
                "ui.lab.detail",
                "A direct Windows renderer interpreting Anodrel's bounded layout tree.",
                16,
            ),
            actions,
            text("ui.lab.status", "Latest semantic event: none", 14),
            text(
                "ui.lab.boundary",
                "An action reports only its ID. It cannot call Windows or grant a capability.",
                13,
            ),
        ],
    ))
    .expect("fixed UI Lab document is valid")
}

fn text(id: &str, value: &str, font_size: u16) -> UiNode {
    UiNode::Text(
        Text::new(
            ElementId::new(id).expect("fixed UI Lab ID is valid"),
            value,
            font_size,
        )
        .expect("fixed UI Lab text is valid"),
    )
}

fn action(id: &str, label: &str) -> UiNode {
    UiNode::Action(
        Action::new(
            ElementId::new(id).expect("fixed UI Lab ID is valid"),
            label,
            15,
            true,
        )
        .expect("fixed UI Lab action is valid"),
    )
}

fn stack(id: &str, axis: Axis, padding: Insets, gap: u16, children: Vec<UiNode>) -> UiNode {
    UiNode::Stack(
        Stack::new(
            ElementId::new(id).expect("fixed UI Lab ID is valid"),
            axis,
            padding,
            gap,
            children,
        )
        .expect("fixed UI Lab stack is valid"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
