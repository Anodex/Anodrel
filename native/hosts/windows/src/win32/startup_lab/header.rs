//! Startup Lab header rendering.

use super::*;

pub(in crate::win32) fn draw_header(canvas: &mut Canvas, layout: &Layout, progress: f32) {
    if progress <= 0.0 {
        return;
    }
    canvas.fill_rect(
        Rect::new(0.0, 0.0, layout.width, layout.header_height),
        &Paint::solid(palette::CHROME.with_alpha(215)),
    );
    canvas.fill_rect(
        Rect::new(
            0.0,
            layout.header_height - layout.unit(1.0),
            layout.width,
            layout.header_height,
        ),
        &Paint::solid(palette::PANEL_EDGE.with_alpha(160)),
    );

    let baseline = layout.header_height / 2.0;
    let wordmark = TextSpec::new("ANODREL", layout.font(30.0), WEIGHT_BOLD)
        .tracked(layout.unit(2.5).round() as i32);
    let wordmark_width = text::width(&wordmark);
    let wordmark_left = layout.margin;
    let wordmark_top = baseline - text::line_height(&wordmark) / 2.0;

    // The wordmark carries the mark's own ramp, so identity reads the same in
    // type as it does in geometry.
    text::draw(
        canvas,
        &wordmark,
        point(wordmark_left, wordmark_top),
        Align::Left,
        &Paint::linear(
            point(wordmark_left, 0.0),
            point(wordmark_left + wordmark_width, 0.0),
            palette::mark_ramp()
                .map(|(position, color)| Stop::new(position, color.scale_alpha(progress)))
                .to_vec(),
        ),
    );

    let divider_x = wordmark_left + wordmark_width + layout.unit(28.0);
    canvas.fill_rect(
        Rect::new(
            divider_x,
            baseline - layout.unit(13.0),
            divider_x + layout.unit(1.0),
            baseline + layout.unit(13.0),
        ),
        &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
    );

    let tagline = TextSpec::new(
        "Native Application Platform",
        layout.font(15.0),
        WEIGHT_REGULAR,
    );
    text::draw(
        canvas,
        &tagline,
        point(
            divider_x + layout.unit(26.0),
            baseline - text::line_height(&tagline) / 2.0,
        ),
        Align::Left,
        &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
    );

    let context = TextSpec::new("Windows Foundation", layout.font(15.0), WEIGHT_REGULAR);
    let surface = TextSpec::new("Startup Lab", layout.font(15.0), WEIGHT_MEDIUM);
    let separator = TextSpec::new(" / ", layout.font(15.0), WEIGHT_REGULAR);
    let total = text::width(&context) + text::width(&separator) + text::width(&surface);
    let right_top = baseline - text::line_height(&context) / 2.0;
    let mut cursor = layout.width - layout.margin - total;

    canvas.fill_circle(
        point(cursor - layout.unit(18.0), baseline),
        layout.unit(4.0),
        &Paint::solid(palette::VIOLET.scale_alpha(progress)),
    );
    cursor = text::draw_run(
        canvas,
        &context,
        point(cursor, right_top),
        &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
    );
    cursor = text::draw_run(
        canvas,
        &separator,
        point(cursor, right_top),
        &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
    );
    text::draw(
        canvas,
        &surface,
        point(cursor, right_top),
        Align::Left,
        &Paint::solid(palette::BLUE_LIGHT.scale_alpha(progress)),
    );
}
