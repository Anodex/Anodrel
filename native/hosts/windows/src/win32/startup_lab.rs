//! The Anodrel Startup Lab surface.
//!
//! Everything on screen is composed into an Anodrel canvas and presented in one
//! blit. The layout is resolution-independent: it is authored against a base
//! size and scaled, so the same code serves a 100% and a 200% display.
//!
//! The screen states only what the host actually verified. Cards report checks
//! that ran during startup; action tiles that are not yet backed by a documented
//! host operation are drawn in a `planned` state rather than being presented as
//! working.

use std::cell::RefCell;

use anodrel_brand::{Icon, mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Color, Mask, Paint, Path, Point, Rect, Stop, point};

use super::text::{Align, TextSpec};
pub(super) mod ambient;
mod animation;
mod backdrop;
mod model;

use super::{StartupLab, text};
pub(super) use ambient::{ambient_region, draw_ambient, invalidate_caches};
use ambient::{draw_ambient_layers, draw_settled};
pub(super) use animation::draw;
pub(super) use backdrop::draw_backdrop;
pub(super) use model::*;

fn draw_header(canvas: &mut Canvas, layout: &Layout, progress: f32) {
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

/// Draws the mark below the title, identity, and validation pill.
fn draw_hero_mark(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
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
fn draw_hero_details(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, elapsed_millis: u64) {
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

fn draw_cards(canvas: &mut Canvas, layout: &Layout, elapsed_millis: u64) {
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
fn card_badge(layout: &Layout, rect: Rect) -> Rect {
    Rect::from_size(
        rect.left + layout.unit(CARD_BADGE_INSET),
        rect.top + layout.unit(CARD_BADGE_INSET),
        layout.unit(CARD_BADGE_SIZE),
        layout.unit(CARD_BADGE_SIZE),
    )
}

/// Returns the top of a card's status line.
fn card_status_top(layout: &Layout, rect: Rect) -> f32 {
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

fn draw_actions(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, elapsed_millis: u64) {
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

fn draw_footer(canvas: &mut Canvas, layout: &Layout, lab: &StartupLab, progress: f32) {
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

#[cfg(test)]
mod tests;
