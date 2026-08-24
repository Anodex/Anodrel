//! Startup Lab action-tile rendering.

use super::*;

pub(in crate::win32) fn draw_actions(
    canvas: &mut Canvas,
    layout: &Layout,
    lab: &StartupLab,
    elapsed_millis: u64,
) {
    let progress = stage(elapsed_millis, 780.0, 400.0);
    if progress <= 0.0 {
        return;
    }
    let container = layout
        .actions
        .translate(0.0, rise(progress, layout.unit(16.0)));
    let radius = layout.unit(14.0);
    canvas.fill_rounded_rect(
        container,
        radius,
        &Paint::solid(palette::PANEL.with_alpha(200).scale_alpha(progress)),
    );
    canvas.stroke_rounded_rect(
        container,
        radius,
        layout.unit(1.0).max(1.0),
        &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
    );

    for (index, action) in ACTIONS.iter().enumerate() {
        let slot = layout
            .action_rect(index)
            .translate(0.0, rise(progress, layout.unit(16.0)));
        let live = tile_is_live(action, lab);
        let hovered = lab.hovered == Some(index) && live;
        let dim = if live { 1.0 } else { 0.45 };

        if hovered {
            canvas.fill_rounded_rect(
                slot.inflate(-layout.unit(4.0)),
                layout.unit(11.0),
                &Paint::solid(action.accent.with_alpha(30).scale_alpha(progress)),
            );
            canvas.stroke_rounded_rect(
                slot.inflate(-layout.unit(4.0)),
                layout.unit(11.0),
                layout.unit(1.0).max(1.0),
                &Paint::solid(action.accent.with_alpha(130).scale_alpha(progress)),
            );
        }

        if index > 0 {
            canvas.fill_rect(
                Rect::new(
                    slot.left,
                    slot.top + layout.unit(22.0),
                    slot.left + layout.unit(1.0),
                    slot.bottom - layout.unit(22.0),
                ),
                &Paint::solid(palette::PANEL_EDGE.with_alpha(150).scale_alpha(progress)),
            );
        }

        let badge_size = layout.unit(44.0);
        let badge = Rect::from_size(
            slot.left + layout.unit(22.0),
            slot.center().y - badge_size / 2.0,
            badge_size,
            badge_size,
        );
        canvas.fill_circle(
            badge.center(),
            badge_size / 2.0,
            &Paint::solid(
                action
                    .accent
                    .with_alpha(if hovered { 44 } else { 24 })
                    .scale_alpha(progress * dim),
            ),
        );
        action.icon.draw(
            canvas,
            badge.inflate(-layout.unit(12.0)),
            layout.unit(1.7).max(1.0),
            &Paint::solid(action.accent.scale_alpha(progress * dim)),
        );

        let text_left = badge.right + layout.unit(15.0);
        let (title_top, subtitle_top) = tile_text_rows(layout, slot);
        let marker_at = tile_marker(layout, slot, live);
        let title = TextSpec::new(action.title, layout.font(16.0), WEIGHT_MEDIUM);
        let subtitle = TextSpec::new(
            tile_subtitle(action, lab),
            layout.font(12.0),
            WEIGHT_REGULAR,
        );
        // Neither line wraps or ellipsizes, so one that does not fit is painted
        // over its marker. `every_tile_label_fits_its_slot_at_the_smallest_supported_size`
        // is the guard; these hold the same line at the point of drawing, so a
        // label added later is caught by any development run rather than only
        // by remembering to extend that test.
        debug_assert!(
            text_left + text::width(&title) <= marker_at.title_limit,
            "{:?} overruns its tile",
            action.title
        );
        debug_assert!(
            text_left + text::width(&subtitle) <= marker_at.subtitle_limit,
            "{:?} overruns its tile",
            tile_subtitle(action, lab)
        );

        text::draw(
            canvas,
            &title,
            point(text_left, title_top),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(progress * dim)),
        );
        text::draw(
            canvas,
            &subtitle,
            point(text_left, subtitle_top),
            Align::Left,
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress * dim)),
        );

        if live {
            draw_chevron(
                canvas,
                point(marker_at.anchor, slot.center().y),
                layout.unit(5.0),
                layout.unit(1.6).max(1.0),
                &Paint::solid(
                    if hovered {
                        action.accent
                    } else {
                        palette::INK_MUTED
                    }
                    .scale_alpha(progress),
                ),
            );
        } else {
            // On the subtitle's line rather than centred where the chevron
            // goes. The marker is a word, and the tile that carries it has the
            // longest title on the strip; sharing the title's line meant the
            // title was painted straight through it. The subtitle beside it
            // says what is missing, so the two read together.
            let marker = planned_marker(layout);
            text::draw(
                canvas,
                &marker,
                point(
                    marker_at.anchor,
                    subtitle_top
                        + (text::line_height(&subtitle) - text::line_height(&marker)) / 2.0,
                ),
                Align::Right,
                &Paint::solid(palette::PLANNED.scale_alpha(progress)),
            );
        }
    }
}

fn draw_chevron(canvas: &mut Canvas, at: Point, size: f32, width: f32, paint: &Paint) {
    canvas.draw_polyline(
        &[
            point(at.x - size * 0.5, at.y - size),
            point(at.x + size * 0.5, at.y),
            point(at.x - size * 0.5, at.y + size),
        ],
        width,
        paint,
    );
}
