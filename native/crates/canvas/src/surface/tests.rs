//! Focused verification for software-canvas pixels, masks, and rasterization.

use super::{Canvas, Mask};
use crate::color::Color;
use crate::geometry::{Rect, point};
use crate::paint::Paint;
use crate::path::Path;

#[test]
fn repositioning_a_mask_moves_its_coverage_without_rebuilding_it() {
    let mut mask = Mask::new(0, 0, 4, 4);
    mask.fill_path(&Path::rect(Rect::new(1.0, 1.0, 3.0, 3.0)));
    let moved = mask.positioned(20, 30);
    mask.reposition(20, 30);

    let mut from_copy = Canvas::new(40, 40);
    let mut from_move = Canvas::new(40, 40);
    from_copy.fill_mask(&moved, &Paint::solid(Color::WHITE));
    from_move.fill_mask(&mask, &Paint::solid(Color::WHITE));
    assert_eq!(from_copy.pixels(), from_move.pixels());
    assert_eq!(from_move.pixel(21, 31), Color::WHITE);
    assert_eq!(from_move.pixel(1, 1), Color::TRANSPARENT);
}

#[test]
fn offset_compositing_reuses_one_mask_without_changing_its_origin() {
    let mask = Mask::from_coverage(1, 1, 1, 1, vec![1.0]).expect("coverage length fits");
    let mut moved = Canvas::new(5, 5);
    moved.fill_mask_offset(&mask, 2, 1, &Paint::solid(Color::WHITE));
    assert_eq!(moved.pixel(3, 2), Color::WHITE);
    assert_eq!(moved.pixel(1, 1), Color::TRANSPARENT);

    let mut original = Canvas::new(5, 5);
    original.fill_mask(&mask, &Paint::solid(Color::WHITE));
    assert_eq!(original.pixel(1, 1), Color::WHITE);

    moved.fill_mask_offset(&mask, i32::MAX, i32::MIN, &Paint::solid(Color::WHITE));
    assert_eq!(moved.pixel(3, 2), Color::WHITE);
}

#[test]
fn a_reported_box_radius_is_the_one_the_blur_applies() {
    // Radii that round together must blur identically, or a caller keying a
    // retained mask on the reported radius would reuse the wrong blur.
    let build = |radius: f32| {
        let mut mask = Mask::new(0, 0, 24, 24);
        mask.fill_path(&Path::rect(Rect::new(8.0, 8.0, 16.0, 16.0)));
        mask.blur(radius);
        mask
    };
    assert_eq!(Mask::blur_box_radius(5.9), Mask::blur_box_radius(6.1));
    let (near, far) = (build(5.9), build(6.1));
    for y in 0..24 {
        for x in 0..24 {
            assert_eq!(near.coverage_at(x, y), far.coverage_at(x, y));
        }
    }
    assert_ne!(Mask::blur_box_radius(6.1), Mask::blur_box_radius(9.1));
    assert_eq!(Mask::blur_box_radius(0.1), 1, "a blur never vanishes");
}

#[test]
fn a_pixel_aligned_rectangle_fills_exactly_its_pixels() {
    let mut canvas = Canvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.fill_rect(Rect::new(2.0, 2.0, 6.0, 6.0), &Paint::solid(Color::WHITE));
    assert_eq!(canvas.pixel(1, 3), Color::BLACK);
    assert_eq!(canvas.pixel(2, 3), Color::WHITE);
    assert_eq!(canvas.pixel(5, 3), Color::WHITE);
    assert_eq!(canvas.pixel(6, 3), Color::BLACK);
}

#[test]
fn a_half_covered_pixel_receives_half_coverage() {
    let mut canvas = Canvas::new(4, 4);
    canvas.clear(Color::BLACK);
    canvas.fill_rect(Rect::new(0.0, 0.0, 1.5, 4.0), &Paint::solid(Color::WHITE));
    assert_eq!(canvas.pixel(0, 0), Color::WHITE);
    let partial = canvas.pixel(1, 0);
    assert!(
        (i16::from(partial.red) - 128).abs() <= 2,
        "expected roughly half coverage, got {partial:?}"
    );
}

#[test]
fn a_new_canvas_starts_fully_transparent() {
    let canvas = Canvas::new(4, 4);
    assert_eq!(canvas.pixel(2, 2).alpha, 0);
}

#[test]
fn compositing_onto_a_transparent_canvas_accumulates_alpha() {
    let mut canvas = Canvas::new(4, 4);
    canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
    let once = canvas.pixel(1, 1);
    assert!((i16::from(once.alpha) - 128).abs() <= 2, "got {once:?}");
    // The colour is preserved rather than being dragged toward black.
    assert_eq!(once.red, 255);
    canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
    assert!(canvas.pixel(1, 1).alpha > once.alpha);
}

#[test]
fn drawing_outside_the_canvas_is_clipped_not_panicking() {
    let mut canvas = Canvas::new(4, 4);
    canvas.clear(Color::BLACK);
    canvas.fill_rect(
        Rect::new(-100.0, -100.0, -50.0, -50.0),
        &Paint::solid(Color::WHITE),
    );
    canvas.fill_rect(
        Rect::new(500.0, 500.0, 900.0, 900.0),
        &Paint::solid(Color::WHITE),
    );
    canvas.fill_rect(
        Rect::new(-10.0, -10.0, 2.0, 2.0),
        &Paint::solid(Color::WHITE),
    );
    assert_eq!(canvas.pixel(0, 0), Color::WHITE);
    assert_eq!(canvas.pixel(3, 3), Color::BLACK);
}

#[test]
fn a_ring_leaves_its_centre_untouched() {
    let mut canvas = Canvas::new(64, 64);
    canvas.clear(Color::BLACK);
    canvas.fill_path(
        &Path::ring(point(32.0, 32.0), 24.0, 6.0),
        &Paint::solid(Color::WHITE),
    );
    assert_eq!(canvas.pixel(32, 32), Color::BLACK);
    assert_eq!(canvas.pixel(32, 10), Color::WHITE);
}

#[test]
fn opacity_composites_against_the_backdrop() {
    let mut canvas = Canvas::new(4, 4);
    canvas.clear(Color::BLACK);
    canvas.fill_rect(canvas.bounds(), &Paint::solid(Color::WHITE.with_alpha(128)));
    let blended = canvas.pixel(1, 1);
    assert!((i16::from(blended.red) - 128).abs() <= 2);
    assert_eq!(blended.alpha, 255);
}

#[test]
fn compositing_a_layer_respects_an_explicit_destination_clip() {
    let mut source = Canvas::new(4, 4);
    source.clear(Color::WHITE);
    let mut target = Canvas::new(6, 6);
    target.clear(Color::BLACK);

    target.draw_canvas_clipped(&source, 1, 1, 1.0, Rect::new(2.0, 2.0, 4.0, 4.0));

    assert_eq!(target.pixel(1, 1), Color::BLACK);
    assert_eq!(target.pixel(2, 2), Color::WHITE);
    assert_eq!(target.pixel(3, 3), Color::WHITE);
    assert_eq!(target.pixel(4, 4), Color::BLACK);
}

#[test]
fn a_gradient_varies_across_the_surface() {
    let mut canvas = Canvas::new(64, 4);
    canvas.fill_rect(
        canvas.bounds(),
        &Paint::horizontal(0.0, 64.0, Color::BLACK, Color::WHITE),
    );
    assert!(canvas.pixel(1, 1).red < canvas.pixel(32, 1).red);
    assert!(canvas.pixel(32, 1).red < canvas.pixel(62, 1).red);
}

#[test]
fn blurring_spreads_coverage_beyond_the_original_edge() {
    let path = Path::rect(Rect::new(20.0, 20.0, 40.0, 40.0));
    let mut mask = Mask::for_path(&path, 12.0);
    let outside_x = (18 - (20 - 12)) as u32;
    assert_eq!(mask.coverage_at(outside_x, 20), 0.0);
    mask.blur(6.0);
    assert!(mask.coverage_at(outside_x, 20) > 0.0);
}

#[test]
fn blurring_conserves_the_bulk_of_its_energy() {
    let path = Path::rect(Rect::new(30.0, 30.0, 60.0, 60.0));
    let mut mask = Mask::for_path(&path, 24.0);
    let before: f32 = (0..mask.height())
        .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
        .map(|(x, y)| mask.coverage_at(x, y))
        .sum();
    mask.blur(8.0);
    let after: f32 = (0..mask.height())
        .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
        .map(|(x, y)| mask.coverage_at(x, y))
        .sum();
    assert!(
        (after / before - 1.0).abs() < 0.05,
        "blur lost energy: {before} -> {after}"
    );
}

#[test]
fn a_glow_darkens_with_distance_from_the_shape() {
    let mut canvas = Canvas::new(120, 120);
    canvas.clear(Color::BLACK);
    let path = Path::circle(point(60.0, 60.0), 20.0);
    canvas.draw_glow(&path, 14.0, 2, &Paint::solid(Color::WHITE.with_alpha(160)));
    let near = canvas.pixel(60, 34).red;
    let far = canvas.pixel(60, 20).red;
    assert!(near > far, "glow should fall off: near {near}, far {far}");
}

#[test]
fn sampling_is_clamped_to_the_canvas() {
    let mut canvas = Canvas::new(4, 4);
    canvas.clear(Color::rgb(10, 20, 30));
    assert_eq!(canvas.sample(point(-500.0, -500.0)), Color::rgb(10, 20, 30));
    assert_eq!(canvas.sample(point(500.0, 500.0)), Color::rgb(10, 20, 30));
}

#[test]
fn a_stroke_covers_the_edge_and_spares_the_interior() {
    let mut canvas = Canvas::new(64, 64);
    canvas.clear(Color::BLACK);
    canvas.stroke_rounded_rect(
        Rect::new(10.0, 10.0, 54.0, 54.0),
        8.0,
        4.0,
        &Paint::solid(Color::WHITE),
    );
    assert_eq!(canvas.pixel(32, 32), Color::BLACK);
    assert!(canvas.pixel(32, 10).red > 200);
}
