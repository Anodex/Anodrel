//! Recursive semantic-node painting and the host-owned scrollbar overlay.

use super::{
    controls::{draw_action, draw_field},
    text_render::{TextRender, draw_text},
    *,
};

// Recursive rendering takes the exact immutable state for one paint pass.
// Keeping those dependencies visible prevents a process-global render context.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_node(
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
pub(super) fn draw_scrollbar(
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
