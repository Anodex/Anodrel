//! Software-canvas rendering for the retained Windows UI Lab.
//!
//! The view state and interaction routes remain in the parent module. This
//! module consumes one immutable layout and host-owned state for a paint pass.

use super::*;

mod controls;
mod node;
mod text_render;

use node::{draw_node, draw_scrollbar};

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
    if let Some((scrollbar, _)) = lab.first_scrollbar_in_layout(&layout) {
        draw_scrollbar(canvas, &scrollbar, surface, palette);
    }
}

/// Concrete host colours for one UI-Lab paint pass.
///
/// The portable UI document chooses only semantic roles. Windows chooses this
/// palette, substituting direct system colours only while high contrast is on.
#[derive(Clone, Copy)]
pub(super) struct UiLabPalette {
    pub(super) backdrop: Color,
    pub(super) backdrop_lift: Color,
    pub(super) panel: Color,
    pub(super) panel_raised: Color,
    pub(super) panel_edge: Color,
    pub(super) ink: Color,
    pub(super) ink_soft: Color,
    pub(super) accent_shell: Color,
    pub(super) accent_core: Color,
    pub(super) accent_ipc: Color,
    pub(super) accent_text: Color,
    pub(super) button_text: Color,
    pub(super) scrollbar_track: Color,
    pub(super) scrollbar_thumb: Color,
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
            scrollbar_track: palette::PANEL_EDGE,
            scrollbar_thumb: palette::INK_MUTED,
        }
    }

    pub(super) fn high_contrast(colors: SystemColors) -> Self {
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
            scrollbar_track: button_face,
            scrollbar_thumb: window_text,
        }
    }
}

const fn color(value: Rgb) -> Color {
    Color::rgb(value.red, value.green, value.blue)
}

/// Base-space layout and its scale into a real client area.
#[derive(Clone, Copy)]
pub(super) struct Surface {
    pub(super) scale: f32,
    width: f32,
    height: f32,
}

impl Surface {
    pub(super) fn new(width: f32, height: f32) -> Self {
        let scale = (width / BASE_WIDTH)
            .min(height / BASE_HEIGHT)
            .clamp(0.6, 4.0);
        Self {
            scale,
            width,
            height,
        }
    }

    pub(super) fn bounds(self) -> UiRect {
        UiRect::from_size(0.0, 0.0, self.width / self.scale, self.height / self.scale)
    }

    pub(super) fn to_canvas_rect(self, bounds: UiRect) -> Rect {
        Rect::new(
            bounds.left * self.scale,
            bounds.top * self.scale,
            bounds.right * self.scale,
            bounds.bottom * self.scale,
        )
    }

    pub(super) fn to_ui_point(self, at: Point) -> UiPoint {
        UiPoint::new(at.x / self.scale, at.y / self.scale)
    }

    pub(super) fn font(self, size: u16) -> i32 {
        (f32::from(size) * self.scale).round().max(1.0) as i32
    }
}

/// The Windows seam required by the portable UI layout contract.
pub(super) struct WindowsTextMeasurer;

impl TextMeasurer for WindowsTextMeasurer {
    fn measure(&self, value: &str, font_size: u16) -> UiSize {
        let spec = TextSpec::new(value, i32::from(font_size), WEIGHT_REGULAR);
        UiSize::new(text::width(&spec), text::line_height(&spec))
    }
}

pub(super) fn status_text(lab: &UiLab) -> Option<String> {
    lab.status_target.as_ref()?;
    Some(lab.last_action.as_ref().map_or_else(
        || "Latest semantic event: none".to_owned(),
        |id| format!("Latest semantic event: {id} (no native operation)"),
    ))
}
