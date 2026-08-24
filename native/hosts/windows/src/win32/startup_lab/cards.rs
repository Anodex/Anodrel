//! Startup Lab startup-check card rendering and geometry.

use super::*;

pub(in crate::win32) fn draw_cards(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
    for (index, card) in CARDS.iter().enumerate() {
        let progress = stage(elapsed_millis, 600.0 + index as f32 * 70.0, 460.0);
        if progress <= 0.0 {
            continue;
        }
        let rect = layout
            .card_rect(index)
            .translate(0.0, rise(progress, layout.unit(18.0)));
        let radius = layout.unit(14.0);

        canvas.fill_rounded_rect(
            rect,
            radius,
            &Paint::linear(
                point(0.0, rect.top),
                point(0.0, rect.bottom),
                vec![
                    Stop::new(0.0, palette::PANEL.with_alpha(242).scale_alpha(progress)),
                    Stop::new(1.0, palette::PANEL.with_alpha(190).scale_alpha(progress)),
                ],
            ),
        );
        canvas.stroke_rounded_rect(
            rect,
            radius,
            layout.unit(1.0).max(1.0),
            &Paint::solid(palette::PANEL_EDGE.scale_alpha(progress)),
        );

        let badge = card_badge(layout, rect);
        let badge_size = badge.width();
        canvas.fill_circle(
            badge.center(),
            badge_size / 2.0,
            &Paint::solid(card.accent.with_alpha(28).scale_alpha(progress)),
        );
        canvas.stroke_path(
            &Path::circle(badge.center(), badge_size / 2.0),
            layout.unit(1.0).max(1.0),
            &Paint::solid(card.accent.with_alpha(110).scale_alpha(progress)),
        );
        card.icon.draw(
            canvas,
            badge.inflate(-layout.unit(13.0)),
            layout.unit(1.7).max(1.0),
            &Paint::solid(card.accent.scale_alpha(progress)),
        );

        let text_left = badge.right + layout.unit(16.0);
        let title = TextSpec::new(card.title, layout.font(17.0), WEIGHT_MEDIUM);
        let title_top = rect.top + layout.unit(24.0);
        text::draw(
            canvas,
            &title,
            point(text_left, title_top),
            Align::Left,
            &Paint::solid(palette::INK.scale_alpha(progress)),
        );
        canvas.fill_circle(
            point(
                text_left + text::width(&title) + layout.unit(11.0),
                title_top + text::line_height(&title) / 2.0,
            ),
            layout.unit(4.0),
            &Paint::solid(palette::READY.scale_alpha(progress)),
        );

        // The status and detail lines run the card's full width, so they start
        // below the badge rather than beside it. They must clear it: the badge
        // shares their left edge, so an overlap puts the circle's arc straight
        // through the text.
        let status = TextSpec::new(card.status, layout.font(17.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &status,
            point(rect.left + layout.unit(20.0), card_status_top(layout, rect)),
            Align::Left,
            &Paint::solid(card.accent.scale_alpha(progress)),
        );

        let detail = TextSpec::new(card.detail, layout.font(13.0), WEIGHT_REGULAR);
        text::draw(
            canvas,
            &detail,
            point(rect.left + layout.unit(20.0), rect.top + layout.unit(102.0)),
            Align::Left,
            &Paint::solid(palette::INK_MUTED.scale_alpha(progress)),
        );

        draw_chip(
            canvas,
            layout,
            point(
                rect.left + layout.unit(20.0),
                rect.bottom - layout.unit(32.0),
            ),
            card.badge,
            card.accent,
            progress,
        );
    }
}

/// Inset of a card's icon badge from the card's top-left corner.
const CARD_BADGE_INSET: f32 = 20.0;
/// Diameter of a card's icon badge.
const CARD_BADGE_SIZE: f32 = 46.0;
/// Top of a card's status line, measured from the card's top edge.
const CARD_STATUS_TOP: f32 = 74.0;

/// Returns a card's icon badge circle.
pub(in crate::win32) fn card_badge(layout: &Layout, rect: Rect) -> Rect {
    Rect::from_size(
        rect.left + layout.unit(CARD_BADGE_INSET),
        rect.top + layout.unit(CARD_BADGE_INSET),
        layout.unit(CARD_BADGE_SIZE),
        layout.unit(CARD_BADGE_SIZE),
    )
}

/// Returns the top of a card's status line.
pub(in crate::win32) fn card_status_top(layout: &Layout, rect: Rect) -> f32 {
    rect.top + layout.unit(CARD_STATUS_TOP)
}

fn draw_chip(
    canvas: &mut Canvas,
    layout: &Layout,
    at: Point,
    label: &str,
    accent: Color,
    progress: f32,
) {
    let spec = TextSpec::new(label, layout.font(11.0), WEIGHT_SEMIBOLD)
        .tracked(layout.unit(0.6).round() as i32);
    let padding = layout.unit(9.0);
    let height = layout.unit(21.0);
    let rect = Rect::from_size(at.x, at.y, text::width(&spec) + padding * 2.0, height);
    canvas.fill_rounded_rect(
        rect,
        layout.unit(5.0),
        &Paint::solid(accent.with_alpha(30).scale_alpha(progress)),
    );
    text::draw(
        canvas,
        &spec,
        point(
            rect.left + padding,
            rect.center().y - text::line_height(&spec) / 2.0,
        ),
        Align::Left,
        &Paint::solid(accent.scale_alpha(progress)),
    );
}
