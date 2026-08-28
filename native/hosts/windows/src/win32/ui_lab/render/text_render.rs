//! Native text-run resolution and paint operations for the UI Lab.

use super::*;

/// One fully resolved text run ready for the native canvas.
///
/// Keeping the text facts together makes text and semantic status rendering
/// use one short drawing boundary without giving the renderer a document-wide
/// status lookup.
pub(super) struct TextRender<'a> {
    value: &'a str,
    font_size: u16,
    tone: UiTextTone,
    bounds: UiRect,
}

impl<'a> TextRender<'a> {
    pub(super) const fn new(
        value: &'a str,
        font_size: u16,
        tone: UiTextTone,
        bounds: UiRect,
    ) -> Self {
        Self {
            value,
            font_size,
            tone,
            bounds,
        }
    }
}

pub(super) fn draw_text(
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
