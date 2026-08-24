//! Cached Startup Lab backdrop composition.

use super::ambient::BACKDROP;
use super::*;

pub(in crate::win32) fn draw_backdrop(canvas: &mut Canvas, layout: &Layout) {
    let reused = BACKDROP.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .is_some_and(|cached| canvas.copy_from(cached))
    });
    if reused {
        return;
    }
    compose_backdrop(canvas, layout);
    let mut snapshot = Canvas::new(canvas.width(), canvas.height());
    if snapshot.copy_from(canvas) {
        BACKDROP.with(|cache| *cache.borrow_mut() = Some(snapshot));
    }
}

fn compose_backdrop(canvas: &mut Canvas, layout: &Layout) {
    canvas.clear(palette::BACKDROP);

    // A wide, low bloom behind the hero lifts the centre away from the frame.
    canvas.fill_rect(
        canvas.bounds(),
        &Paint::radial(
            point(
                layout.width / 2.0,
                layout.mark.center().y + layout.unit(30.0),
            ),
            layout.width * 0.62,
            vec![
                Stop::new(0.0, palette::BACKDROP_LIFT.with_alpha(235)),
                Stop::new(0.45, palette::BACKDROP_LIFT.with_alpha(90)),
                Stop::new(1.0, Color::TRANSPARENT),
            ],
        ),
    );

    // Faceted planes echoing the mark's geometry. They are kept to the outer
    // corners and to a few units of alpha: at this weight they register as
    // depth, while a straight edge crossing the composition would read as a
    // rendering artefact.
    let facets = [
        [point(0.0, 0.0), point(0.20, 0.0), point(0.0, 0.34)],
        [point(1.0, 0.0), point(0.82, 0.0), point(1.0, 0.30)],
        [point(0.0, 1.0), point(0.0, 0.74), point(0.17, 1.0)],
        [point(1.0, 1.0), point(1.0, 0.78), point(0.86, 1.0)],
    ];
    for triangle in facets {
        canvas.fill_path(
            &Path::polygon(
                triangle.map(|vertex| point(vertex.x * layout.width, vertex.y * layout.height)),
            ),
            &Paint::solid(palette::INDIGO.with_alpha(5)),
        );
    }
}
