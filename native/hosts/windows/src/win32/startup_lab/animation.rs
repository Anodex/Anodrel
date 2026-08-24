//! Reveal-frame composition for the Startup Lab.

use super::*;

/// Draws the whole surface.
pub(in crate::win32) fn draw(canvas: &mut Canvas, lab: &StartupLab, elapsed_millis: u64) {
    let layout = Layout::new(canvas.width() as f32, canvas.height() as f32);

    // Once the reveal has settled, retain the invariant parts of the surface.
    // A full repaint and a partial ambient repaint then start from the exact
    // same base pixels, which keeps the optimization visually invisible.
    if elapsed_millis >= REVEAL_MILLIS && draw_settled(canvas, lab, &layout, elapsed_millis) {
        return;
    }

    draw_backdrop(canvas, &layout);
    draw_header(canvas, &layout, stage(elapsed_millis, 0.0, 340.0));
    draw_hero_mark(canvas, &layout, elapsed_millis);
    draw_hero_details(canvas, &layout, lab, elapsed_millis);
    draw_cards(canvas, &layout, elapsed_millis);
    draw_actions(canvas, &layout, lab, elapsed_millis);
    draw_footer(canvas, &layout, lab, stage(elapsed_millis, 820.0, 360.0));
}
