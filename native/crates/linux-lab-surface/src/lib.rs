#![forbid(unsafe_code)]

//! Fixed first-party canvas for Linux Wayland development diagnostics.
//!
//! This draws no application content and owns no Linux or Wayland resource.

use anodrel_brand::{mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Color, Paint, Rect, point, stop};

/// The fixed Linux diagnostic canvas width.
pub const LAB_WIDTH: u32 = 960;
/// The fixed Linux diagnostic canvas height.
pub const LAB_HEIGHT: u32 = 640;

/// Composes the fixed Linux diagnostic canvas in its initial or completed state.
#[must_use]
pub fn compose(activated: bool) -> Canvas {
    let mut canvas = Canvas::new(LAB_WIDTH, LAB_HEIGHT);
    canvas.clear(palette::BACKDROP);
    let bounds = canvas.bounds();
    canvas.fill_rect(
        bounds,
        &Paint::radial(
            point(LAB_WIDTH as f32 * 0.52, LAB_HEIGHT as f32 * 0.32),
            LAB_WIDTH as f32 * 0.8,
            vec![
                stop(0.0, palette::INDIGO.with_alpha(86)),
                stop(0.54, palette::BACKDROP.with_alpha(24)),
                stop(1.0, Color::TRANSPARENT),
            ],
        ),
    );
    let panel = Rect::new(156.0, 96.0, 804.0, 544.0);
    canvas.fill_rounded_rect(panel, 34.0, &Paint::Solid(palette::PANEL_RAISED));
    canvas.stroke_rounded_rect(panel, 34.0, 1.0, &Paint::Solid(palette::PANEL_EDGE));
    mark::draw(
        &mut canvas,
        Rect::new(368.0, 158.0, 592.0, 382.0),
        MarkStyle::hero(),
    );
    let activation = Rect::new(330.0, 416.0, 630.0, 512.0);
    canvas.fill_rounded_rect(
        activation,
        24.0,
        &Paint::Solid(if activated {
            palette::READY
        } else {
            palette::PANEL
        }),
    );
    canvas.stroke_rounded_rect(
        activation,
        24.0,
        1.0,
        &Paint::Solid(if activated {
            palette::READY
        } else {
            palette::ACCENT_CORE
        }),
    );
    canvas.fill_rounded_rect(
        Rect::new(364.0, 440.0, 596.0, 448.0),
        4.0,
        &Paint::Solid(if activated {
            palette::BACKDROP
        } else {
            palette::ACCENT_CORE
        }),
    );
    canvas.fill_rounded_rect(
        Rect::new(330.0, 464.0, 630.0, 470.0),
        3.0,
        &Paint::Solid(palette::INK_MUTED),
    );
    canvas
}

#[cfg(test)]
mod tests {
    use super::{LAB_HEIGHT, LAB_WIDTH, compose};

    #[test]
    fn diagnostic_surface_is_fixed_and_non_empty() {
        let canvas = compose(false);
        assert_eq!((canvas.width(), canvas.height()), (LAB_WIDTH, LAB_HEIGHT));
        assert!(canvas.pixels().iter().any(|pixel| *pixel != 0));
    }
}
