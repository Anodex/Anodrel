//! Linux-only direct Wayland presentation check for the Anodrel Linux Lab.

use std::error::Error;

use anodrel_brand::{mark, mark::MarkStyle, palette};
use anodrel_canvas::{Canvas, Color, Paint, Rect, point, stop};
use anodrel_linux_wayland::{LAB_HEIGHT, LAB_WIDTH, LinuxWaylandLab};

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let mut lab = LinuxWaylandLab::open()?;
    let canvas = compose_lab();
    lab.present(&canvas)?;
    lab.wait_for_close()?;
    Ok(())
}

fn compose_lab() -> Canvas {
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
    canvas.fill_rounded_rect(
        Rect::new(364.0, 432.0, 596.0, 440.0),
        4.0,
        &Paint::Solid(palette::ACCENT_CORE),
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
    use super::compose_lab;
    use anodrel_linux_wayland::{LAB_HEIGHT, LAB_WIDTH};

    #[test]
    fn diagnostic_surface_is_fixed_and_non_empty() {
        let canvas = compose_lab();
        assert_eq!((canvas.width(), canvas.height()), (LAB_WIDTH, LAB_HEIGHT));
        assert!(canvas.pixels().iter().any(|pixel| *pixel != 0));
    }
}
