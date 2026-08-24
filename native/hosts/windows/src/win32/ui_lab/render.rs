//! Software-canvas rendering for the retained Windows UI Lab.
//!
//! The view state and interaction routes remain in the parent module. This
//! module consumes one immutable layout and host-owned state for a paint pass.

use super::*;

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

    fn to_canvas_rect(self, bounds: UiRect) -> Rect {
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

    fn font(self, size: u16) -> i32 {
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
    // Layout retains a bounded semantic record for fully clipped nodes so the
    // accessibility tree can navigate to them. Rendering must still avoid
    // allocating or rasterizing work no viewport can display.
    if item.bounds().is_empty() {
        return;
    }
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
            // A lab replacement is not the value layout measured, so a longer
            // result could paint past this box. The lab uses only short fixed
            // results; a document's own text is never substituted.
            let value = if status_target.is_some_and(|target| target == text_node.id()) {
                status.unwrap_or(text_node.value())
            } else {
                text_node.value()
            };
            draw_text(
                canvas,
                TextRender::new(value, text_node.font_size(), text_node.tone(), bounds),
                surface,
                palette,
            );
        }
        UiNode::Status(status_node) => {
            draw_text(
                canvas,
                TextRender::new(
                    status_node.value(),
                    status_node.font_size(),
                    status_node.tone(),
                    bounds,
                ),
                surface,
                palette,
            );
        }
        UiNode::Action(action) => draw_action(canvas, lab, action, bounds, surface, palette),
        UiNode::Field(field) => draw_field(canvas, lab, field, bounds, surface, palette),
    }
}

/// Draws the one host-owned scrollbar overlay after its clipped document content.
fn draw_scrollbar(
    canvas: &mut Canvas,
    scrollbar: &Scrollbar,
    surface: Surface,
    palette: UiLabPalette,
) {
    let track = surface.to_canvas_rect(scrollbar.track());
    let thumb = surface.to_canvas_rect(scrollbar.thumb());
    let track_radius = (track.width().min(track.height()) / 2.0).max(1.0);
    let thumb_radius = (thumb.width().min(thumb.height()) / 2.0).max(1.0);
    canvas.fill_rounded_rect(track, track_radius, &Paint::solid(palette.scrollbar_track));
    canvas.fill_rounded_rect(thumb, thumb_radius, &Paint::solid(palette.scrollbar_thumb));
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

/// One fully resolved text run ready for the native canvas.
///
/// Keeping the text facts together makes text and semantic status rendering
/// use one short drawing boundary without giving the renderer a document-wide
/// status lookup.
struct TextRender<'a> {
    value: &'a str,
    font_size: u16,
    tone: UiTextTone,
    bounds: UiRect,
}

impl<'a> TextRender<'a> {
    const fn new(value: &'a str, font_size: u16, tone: UiTextTone, bounds: UiRect) -> Self {
        Self {
            value,
            font_size,
            tone,
            bounds,
        }
    }
}

fn draw_text(
    canvas: &mut Canvas,
    text_run: TextRender<'_>,
    surface: Surface,
    palette: UiLabPalette,
) {
    let color = match text_run.tone {
        UiTextTone::Primary => palette.ink,
        UiTextTone::Secondary => palette.ink_soft,
        UiTextTone::Accent => palette.accent_shell,
    };

    // Wrapped with the same function and the same measurer that produced these
    // bounds, in the same logical space, so the painted lines are the lines the
    // layout measured. `bounds` is the widest line, and greedy breaking gives
    // the same result at that width as at the column it was wrapped against.
    //
    let lines = wrap_text(
        text_run.value,
        text_run.font_size,
        text_run.bounds.width(),
        &WindowsTextMeasurer,
    );
    // Advancing by the measured block height divided by its line count keeps
    // the run inside the box the layout reserved, instead of accumulating a
    // per-line rounding difference down a paragraph.
    let step = (text_run.bounds.height() * surface.scale) / lines.len() as f32;
    let font = surface.font(text_run.font_size);
    let left = text_run.bounds.left * surface.scale;
    let top = text_run.bounds.top * surface.scale;
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        text::draw(
            canvas,
            &TextSpec::new(*line, font, WEIGHT_REGULAR),
            point(left, top + step * index as f32),
            Align::Left,
            &Paint::solid(color),
        );
    }
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

pub(super) fn status_text(lab: &UiLab) -> Option<String> {
    lab.status_target.as_ref()?;
    Some(lab.last_action.as_ref().map_or_else(
        || "Latest semantic event: none".to_owned(),
        |id| format!("Latest semantic event: {id} (no native operation)"),
    ))
}
