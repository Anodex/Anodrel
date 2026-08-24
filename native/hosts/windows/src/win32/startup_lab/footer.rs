//! Startup Lab runtime-footnote rendering.

use super::*;

pub(in crate::win32) fn draw_footer(
    canvas: &mut Canvas,
    layout: &Layout,
    lab: &StartupLab,
    progress: f32,
) {
    if progress <= 0.0 {
        return;
    }
    let top = layout.footer_top;
    canvas.fill_rect(
        Rect::new(
            layout.margin,
            top,
            layout.width - layout.margin,
            top + layout.unit(1.0),
        ),
        &Paint::solid(palette::PANEL_EDGE.with_alpha(140).scale_alpha(progress)),
    );

    let row = top + layout.unit(26.0);
    let entries: [(Icon, &str, String, Color); 4] = [
        (
            Icon::Core,
            "Runtime",
            format!("v{}", env!("CARGO_PKG_VERSION")),
            palette::INK_SOFT,
        ),
        (
            Icon::Shell,
            "Memory",
            format!("{:.1} MB", lab.working_set_bytes as f32 / (1024.0 * 1024.0)),
            palette::BLUE_LIGHT,
        ),
        (
            Icon::Diagnostics,
            "Startup",
            // Foundation checks finish in single-digit milliseconds, which
            // rounds to nothing in seconds. Report the unit that has digits.
            if lab.startup_millis < 1_000 {
                format!("{} ms", lab.startup_millis)
            } else {
                format!("{:.2} s", lab.startup_millis as f32 / 1000.0)
            },
            palette::ACCENT_CORE,
        ),
        (Icon::Package, "Integrity", "OK".to_owned(), palette::READY),
    ];

    let mut cursor = layout.margin;
    for (index, (icon, label, value, tone)) in entries.into_iter().enumerate() {
        if index > 0 {
            canvas.fill_rect(
                Rect::new(
                    cursor,
                    row - layout.unit(9.0),
                    cursor + layout.unit(1.0),
                    row + layout.unit(19.0),
                ),
                &Paint::solid(palette::PANEL_EDGE.with_alpha(130).scale_alpha(progress)),
            );
            cursor += layout.unit(26.0);
        }
        let glyph = Rect::from_size(
            cursor,
            row - layout.unit(2.0),
            layout.unit(15.0),
            layout.unit(15.0),
        );
        icon.draw(
            canvas,
            glyph,
            layout.unit(1.3).max(1.0),
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
        );
        cursor = glyph.right + layout.unit(10.0);

        let name = TextSpec::new(label, layout.font(13.0), WEIGHT_REGULAR);
        cursor = text::draw_run(
            canvas,
            &name,
            point(cursor, row),
            &Paint::solid(palette::INK_SOFT.scale_alpha(progress)),
        ) + layout.unit(10.0);

        let reading = TextSpec::new(value, layout.font(13.0), WEIGHT_MEDIUM);
        cursor = text::draw_run(
            canvas,
            &reading,
            point(cursor, row),
            &Paint::solid(tone.scale_alpha(progress)),
        ) + layout.unit(26.0);
    }

    // The right-hand reading is the renderer describing itself: the previous
    // frame's cost, measured by the host that drew it.
    let renderer = TextSpec::new(
        format!(
            "SOFTWARE RENDERER  ·  {}×{}  ·  {:.1} ms",
            canvas.width(),
            canvas.height(),
            lab.last_frame_micros as f32 / 1000.0
        ),
        layout.font(12.0),
        WEIGHT_MEDIUM,
    )
    .tracked(layout.unit(0.4).round() as i32);
    text::draw(
        canvas,
        &renderer,
        point(layout.width - layout.margin, row),
        Align::Right,
        &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
    );
}
