//! Startup Lab hero-mark and identity rendering.

use super::*;

/// Draws the mark below the title, identity, and validation pill.
pub(in crate::win32) fn draw_hero_mark(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
    let mark_progress = stage(elapsed_millis, 120.0, 660.0);
    if mark_progress >= 1.0 {
        // A full paint already has its base pixels. The partial route restores
        // them before it reaches these same layers, so both paths produce the
        // same mark without drawing foreground text twice.
        if ambient_region(layout.width, layout.height).is_none()
            || !draw_ambient_layers(canvas, elapsed_millis)
        {
            mark::draw(canvas, layout.mark, MarkStyle::hero());
        }
    } else if mark_progress > 0.0 {
        // The mark settles from slightly small, so it reads as arriving rather
        // than simply appearing.
        let scale = 0.94 + 0.06 * mark_progress;
        let bounds = Rect::centered(
            layout.mark.center(),
            layout.mark.width() * scale,
            layout.mark.height() * scale,
        );
        mark::draw(
            canvas,
            bounds,
            MarkStyle::hero().with_opacity(mark_progress),
        );
    }
}

/// Draws the settled foreground details that sit above the mark glow.
pub(in crate::win32) fn draw_hero_details(
    canvas: &mut Canvas,
    layout: &Layout,
    lab: &StartupLab,
    elapsed_millis: u64,
) {
    let center_x = layout.width / 2.0;

    let title_progress = stage(elapsed_millis, 380.0, 440.0);
    if title_progress > 0.0 {
        let title = TextSpec::new(&lab.package.display_name, layout.font(44.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &title,
            point(
                center_x,
                layout.title_baseline + rise(title_progress, layout.unit(14.0)),
            ),
            Align::Center,
            &Paint::solid(palette::INK.scale_alpha(title_progress)),
        );
    }

    let identity_progress = stage(elapsed_millis, 470.0, 440.0);
    if identity_progress > 0.0 {
        let label = TextSpec::new("Validated Application", layout.font(17.0), WEIGHT_REGULAR);
        let separator = TextSpec::new("  /  ", layout.font(17.0), WEIGHT_REGULAR);
        let identifier = TextSpec::new(
            &lab.package.application_id,
            layout.font(17.0),
            WEIGHT_MEDIUM,
        );
        let total = text::width(&label) + text::width(&separator) + text::width(&identifier);
        let top = layout.identity_baseline + rise(identity_progress, layout.unit(12.0));
        let mut cursor = center_x - total / 2.0;
        cursor = text::draw_run(
            canvas,
            &label,
            point(cursor, top),
            &Paint::solid(palette::INK_SOFT.scale_alpha(identity_progress)),
        );
        cursor = text::draw_run(
            canvas,
            &separator,
            point(cursor, top),
            &Paint::solid(palette::INK_MUTED.scale_alpha(identity_progress)),
        );
        text::draw(
            canvas,
            &identifier,
            point(cursor, top),
            Align::Left,
            &Paint::solid(palette::BLUE_LIGHT.scale_alpha(identity_progress)),
        );
    }

    let pill_progress = stage(elapsed_millis, 560.0, 420.0);
    if pill_progress > 0.0 {
        let pill = layout
            .pill
            .translate(0.0, rise(pill_progress, layout.unit(10.0)));
        let radius = pill.height() / 2.0;
        canvas.fill_rounded_rect(
            pill,
            radius,
            &Paint::solid(palette::VIOLET.with_alpha(26).scale_alpha(pill_progress)),
        );
        canvas.stroke_rounded_rect(
            pill,
            radius,
            layout.unit(1.0).max(1.0),
            &Paint::solid(palette::VIOLET.with_alpha(120).scale_alpha(pill_progress)),
        );
        let glyph = Rect::from_size(
            pill.left + layout.unit(15.0),
            pill.center().y - layout.unit(7.5),
            layout.unit(15.0),
            layout.unit(15.0),
        );
        Icon::Package.draw(
            canvas,
            glyph,
            layout.unit(1.4).max(1.0),
            &Paint::solid(palette::VIOLET_LIGHT.scale_alpha(pill_progress)),
        );
        let caption = TextSpec::new("Validated", layout.font(14.0), WEIGHT_MEDIUM);
        text::draw(
            canvas,
            &caption,
            point(
                glyph.right + layout.unit(9.0),
                pill.center().y - text::line_height(&caption) / 2.0,
            ),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(pill_progress)),
        );
    }
}
