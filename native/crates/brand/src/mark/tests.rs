//! Focused verification for Anodrel mark geometry and raster effects.

use super::{
    GLOW_CACHE_LIMIT, GLOWS, MarkStyle, Piece, draw, draw_glow_layer, face_paint, glow_paint,
    pieces, scaled_raster, silhouette,
};
use crate::palette;
use anodrel_canvas::{Canvas, Color, Mask, Rect, point};

const UNIT: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

/// Draws the glow the way it was drawn before the mask was retained:
/// coverage sampled at the exact sub-pixel bounds, blurred, then composited.
///
/// This is the reference the retained mask is held against, so it stays
/// spelled out here rather than sharing code with the path under test.
fn glow_built_in_place(canvas: &mut Canvas, bounds: Rect, style: MarkStyle) {
    let image = scaled_raster(bounds.width().max(bounds.height())).expect("the mark decodes");
    let paint = glow_paint(bounds, style.opacity);
    let radius = bounds.width() * style.glow_ratio;
    let padding = (radius * 1.5).ceil().max(1.0);
    let origin_x = (bounds.left - padding).floor() as i32;
    let origin_y = (bounds.top - padding).floor() as i32;
    let width = (bounds.width() + padding * 2.0).ceil().max(1.0) as u32;
    let height = (bounds.height() + padding * 2.0).ceil().max(1.0) as u32;

    let mut coverage = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height {
        let v = ((origin_y + y as i32) as f32 + 0.5 - bounds.top) / bounds.height();
        for x in 0..width {
            let u = ((origin_x + x as i32) as f32 + 0.5 - bounds.left) / bounds.width();
            coverage.push(if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                f32::from(image.sample_bilinear(u, v).alpha) / 255.0
            } else {
                0.0
            });
        }
    }

    let mut mask = Mask::from_coverage(origin_x, origin_y, width, height, coverage)
        .expect("the coverage matches its size");
    mask.blur(radius);
    for _ in 0..style.glow_passes {
        canvas.fill_mask(&mask, &paint);
    }
}

/// Largest per-channel difference between two canvases of the same size.
fn largest_channel_difference(left: &Canvas, right: &Canvas) -> i16 {
    let mut worst = 0;
    for y in 0..left.height() as i32 {
        for x in 0..left.width() as i32 {
            let (a, b) = (left.pixel(x, y), right.pixel(x, y));
            for (from, to) in [
                (a.red, b.red),
                (a.green, b.green),
                (a.blue, b.blue),
                (a.alpha, b.alpha),
            ] {
                worst = worst.max((i16::from(from) - i16::from(to)).abs());
            }
        }
    }
    worst
}

/// Largest channel difference the retained glow may show against one built
/// at the exact sub-pixel bounds, out of 255.
///
/// The retained mask is placed on whole pixels and sized to whole pixels,
/// so it can sit up to half a pixel from where an exactly built one would.
/// The mark's alpha is spread over a 48-pixel blur before it is composited,
/// which is what turns that half pixel into a handful of levels rather than
/// a visible shift. Measured across quarter-pixel placements the two agree
/// exactly on the grid and differ by at most 7 at the half-pixel worst
/// case, so this is that with a step to spare.
const GLOW_TOLERANCE: i16 = 8;

/// The unquantized glow paint kept only as the approximation reference.
fn exact_glow_paint(bounds: Rect, opacity: f32) -> anodrel_canvas::Paint {
    anodrel_canvas::Paint::linear(
        point(bounds.left, 0.0),
        point(bounds.right, 0.0),
        vec![
            anodrel_canvas::Stop::new(0.0, palette::VIOLET.with_alpha(140).scale_alpha(opacity)),
            anodrel_canvas::Stop::new(0.5, palette::INDIGO.with_alpha(120).scale_alpha(opacity)),
            anodrel_canvas::Stop::new(1.0, palette::BLUE.with_alpha(140).scale_alpha(opacity)),
        ],
    )
}

#[test]
fn the_quantized_glow_paint_stays_within_one_channel_level() {
    for bounds in [
        Rect::new(41.25, 0.0, 261.25, 220.0),
        Rect::new(47.75, 0.0, 412.75, 365.0),
    ] {
        for opacity in [0.17, 0.63, 1.0] {
            let (exact, quantized) = (
                exact_glow_paint(bounds, opacity),
                glow_paint(bounds, opacity),
            );
            for sample in 0..=4_096 {
                let x = bounds.left - 8.0 + sample as f32 * (bounds.width() + 16.0) / 4_096.0;
                let (a, b) = (exact.sample(point(x, 0.5)), quantized.sample(point(x, 0.5)));
                for (exact, approximate) in [
                    (a.red, b.red),
                    (a.green, b.green),
                    (a.blue, b.blue),
                    (a.alpha, b.alpha),
                ] {
                    assert!(
                        (i16::from(exact) - i16::from(approximate)).abs() <= 1,
                        "glow paint exceeds one channel level at {x}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_retained_glow_matches_one_built_in_place() {
    let style = MarkStyle::hero();
    // Every quarter-pixel placement, including the half-pixel worst case.
    for step in 0..8_u8 {
        // Both the position and the size are taken off the pixel grid, so
        // the retained mask is rounded on every axis it can be rounded on.
        let offset = f32::from(step) / 4.0;
        let (left, top, edge) = (46.0 + offset, 38.0 + offset, 220.0 + offset);
        let bounds = Rect::new(left, top, left + edge, top + edge);
        let mut retained = Canvas::new(320, 300);
        let mut in_place = Canvas::new(320, 300);
        // Composited over the opaque backdrop it is drawn over in use. On a
        // transparent canvas the fully transparent pixels around the glow
        // keep whatever colour was blended at zero alpha, which is
        // invisible but not equal.
        retained.clear(Color::rgb(11, 13, 22));
        in_place.clear(Color::rgb(11, 13, 22));

        GLOWS.with(|cache| cache.borrow_mut().clear());
        draw_glow_layer(&mut retained, bounds, style);
        glow_built_in_place(&mut in_place, bounds, style);

        let worst = largest_channel_difference(&retained, &in_place);
        assert!(
            worst <= GLOW_TOLERANCE,
            "at a {offset} pixel offset the retained glow differs by {worst} of 255"
        );
    }
}

#[test]
fn a_reveal_reuses_one_retained_glow() {
    // A reveal changes the mark's size by well under a pixel per frame.
    // Every one of those frames has to land on the same retained mask, or
    // the retention saves nothing.
    GLOWS.with(|cache| cache.borrow_mut().clear());
    let mut canvas = Canvas::new(320, 300);
    for frame in 0..40_u8 {
        let scale = 0.999 + 0.001 * (f32::from(frame) / 39.0);
        let edge = 220.0 * scale;
        let bounds = Rect::new(46.0, 38.0, 46.0 + edge, 38.0 + edge);
        draw_glow_layer(&mut canvas, bounds, MarkStyle::hero());
    }
    assert_eq!(GLOWS.with(|cache| cache.borrow().len()), 1);
}

#[test]
fn retained_glows_stay_bounded() {
    GLOWS.with(|cache| cache.borrow_mut().clear());
    let mut canvas = Canvas::new(900, 900);
    for step in 0..12_u8 {
        let edge = 200.0 + f32::from(step) * 40.0;
        let bounds = Rect::new(10.0, 10.0, 10.0 + edge, 10.0 + edge);
        draw_glow_layer(&mut canvas, bounds, MarkStyle::hero());
        assert!(GLOWS.with(|cache| cache.borrow().len()) <= GLOW_CACHE_LIMIT);
    }
}

#[test]
fn every_piece_stays_inside_the_unit_square() {
    for piece in Piece::ALL {
        let bounds = piece.unit_path().bounds();
        assert!(
            bounds.left >= 0.0 && bounds.top >= 0.0,
            "{piece:?} escapes the top-left"
        );
        assert!(
            bounds.right <= 1.0 && bounds.bottom <= 1.0,
            "{piece:?} escapes the bottom-right"
        );
    }
}

#[test]
fn the_legs_mirror_each_other_about_the_centre_line() {
    let left = Piece::LeftLeg.unit_path().bounds();
    let right = Piece::RightLeg.unit_path().bounds();
    assert!((left.left - (1.0 - right.right)).abs() < 1e-4);
    assert!((left.right - (1.0 - right.left)).abs() < 1e-4);
    assert!((left.top - right.top).abs() < 1e-4);
    assert!((left.bottom - right.bottom).abs() < 1e-4);
}

#[test]
fn the_apex_and_crossbar_are_symmetric_about_the_centre_line() {
    for piece in [Piece::Apex, Piece::Crossbar] {
        let bounds = piece.unit_path().bounds();
        let center = (bounds.left + bounds.right) / 2.0;
        assert!(
            (center - 0.5).abs() < 1e-4,
            "{piece:?} is not centred: {center}"
        );
    }
}

#[test]
fn the_legs_and_crossbar_share_one_baseline() {
    let baseline = Piece::LeftLeg.unit_path().bounds().bottom;
    assert!((Piece::RightLeg.unit_path().bounds().bottom - baseline).abs() < 1e-4);
    assert!((Piece::Crossbar.unit_path().bounds().bottom - baseline).abs() < 1e-4);
}

#[test]
fn the_pieces_are_separated_by_visible_gaps() {
    let apex_bottom = Piece::Apex.unit_path().bounds().bottom;
    let leg_top = Piece::LeftLeg.unit_path().bounds().top;
    assert!(
        leg_top > apex_bottom,
        "apex ends at {apex_bottom} but the leg starts at {leg_top}"
    );
}

#[test]
fn the_silhouette_carries_one_contour_per_piece() {
    assert_eq!(silhouette(UNIT).contours().len(), Piece::ALL.len());
}

#[test]
fn fitting_places_the_mark_inside_its_target() {
    let target = Rect::new(100.0, 40.0, 300.0, 240.0);
    for (piece, path) in pieces(target) {
        let bounds = path.bounds();
        assert!(
            bounds.left >= target.left - 0.01 && bounds.right <= target.right + 0.01,
            "{piece:?} escapes horizontally"
        );
        assert!(
            bounds.top >= target.top - 0.01 && bounds.bottom <= target.bottom + 0.01,
            "{piece:?} escapes vertically"
        );
    }
}

#[test]
fn the_face_gradient_runs_violet_on_the_left_and_blue_on_the_right() {
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let paint = face_paint(bounds);
    let left = paint.sample(point(0.0, 50.0));
    let right = paint.sample(point(100.0, 50.0));
    assert!(left.red > right.red, "the left end should be violet");
    assert!(right.blue > right.red, "the right end should be blue");
}

#[test]
fn drawing_the_mark_paints_inside_its_bounds_and_leaves_the_corners_alone() {
    let mut canvas = Canvas::new(240, 240);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        Rect::new(20.0, 20.0, 220.0, 220.0),
        MarkStyle::compact(),
    );
    // The apex sits on the centre line near the top of the mark.
    assert_ne!(canvas.pixel(120, 60), Color::BLACK);
    // The unit square's corners are outside every piece.
    assert_eq!(canvas.pixel(2, 2), Color::BLACK);
    assert_eq!(canvas.pixel(237, 2), Color::BLACK);
}

#[test]
fn a_fully_faded_mark_draws_nothing() {
    let mut canvas = Canvas::new(120, 120);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        Rect::new(10.0, 10.0, 110.0, 110.0),
        MarkStyle::hero().with_opacity(0.0),
    );
    assert_eq!(canvas.pixel(60, 40), Color::BLACK);
}

#[test]
fn an_empty_target_is_ignored() {
    let mut canvas = Canvas::new(32, 32);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        Rect::new(10.0, 10.0, 10.0, 10.0),
        MarkStyle::hero(),
    );
    assert_eq!(canvas.pixel(10, 10), Color::BLACK);
}

#[test]
fn the_authored_asset_loads_at_its_declared_size() {
    let image = super::raster().expect("the embedded asset matches RASTER_SIDE");
    assert_eq!(image.width(), super::RASTER_SIDE);
    assert_eq!(image.height(), super::RASTER_SIDE);
}

#[test]
fn the_authored_asset_is_a_clean_cut_out() {
    let image = super::raster().expect("asset loads");
    // Corners must be empty, or the mark would sit on a plate.
    for (x, y) in [(0, 0), (511, 0), (0, 511), (511, 511)] {
        assert_eq!(image.pixel(x, y).alpha, 0, "corner ({x},{y}) is not clear");
    }
    // No baked glow: the negative space inside the A stays empty, so the
    // draw-time bloom cannot double up.
    assert_eq!(
        image.pixel(256, 235).alpha,
        0,
        "inner negative space is not clear"
    );
}

#[test]
fn the_artwork_reaches_every_edge_of_the_asset() {
    // The asset is cropped square to its artwork, which is what lets the
    // raster and the geometry share one set of bounds.
    let image = super::raster().expect("asset loads");
    let (left, top, right, bottom) = image.opaque_bounds(3).expect("the asset contains artwork");
    assert!(left <= 2 && top <= 2, "artwork is inset at ({left},{top})");
    assert!(
        right >= super::RASTER_SIDE - 2 && bottom >= super::RASTER_SIDE - 2,
        "artwork stops short at ({right},{bottom})"
    );
}

#[test]
fn the_raster_and_the_geometry_cover_the_same_bounds() {
    // Placement must not shift when the renderer crosses RASTER_MIN_EDGE.
    let painted_extent = |edge: f32| {
        let side = (edge as u32) + 40;
        let mut canvas = Canvas::new(side, side);
        canvas.clear(Color::BLACK);
        let bounds = Rect::new(20.0, 20.0, 20.0 + edge, 20.0 + edge);
        draw(&mut canvas, bounds, MarkStyle::compact());
        let lit: Vec<(i32, i32)> = (0..side as i32)
            .flat_map(|y| (0..side as i32).map(move |x| (x, y)))
            .filter(|(x, y)| canvas.pixel(*x, *y) != Color::BLACK)
            .collect();
        let left = lit.iter().map(|(x, _)| *x).min().expect("drew");
        let right = lit.iter().map(|(x, _)| *x).max().expect("drew");
        (left, right)
    };

    // Just below and just above the threshold, normalised to the same edge.
    let (geometry_left, geometry_right) = painted_extent(super::RASTER_MIN_EDGE - 2.0);
    let (raster_left, raster_right) = painted_extent(super::RASTER_MIN_EDGE + 2.0);
    assert!(
        (geometry_left - raster_left).abs() <= 3,
        "left edge jumps: geometry {geometry_left}, raster {raster_left}"
    );
    assert!(
        (geometry_right - raster_right).abs() <= 5,
        "right edge jumps: geometry {geometry_right}, raster {raster_right}"
    );
}

#[test]
fn a_hero_sized_mark_carries_the_brand_gradient() {
    let mut canvas = Canvas::new(320, 320);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        Rect::new(20.0, 20.0, 300.0, 300.0),
        MarkStyle::compact(),
    );
    // Sample the two legs: violet on the left, blue on the right.
    let left_leg = canvas.pixel(90, 250);
    let right_leg = canvas.pixel(240, 250);
    assert!(
        left_leg.red > left_leg.green + 40,
        "left leg is not violet: {left_leg:?}"
    );
    assert!(
        right_leg.blue > right_leg.red + 40,
        "right leg is not blue: {right_leg:?}"
    );
}
