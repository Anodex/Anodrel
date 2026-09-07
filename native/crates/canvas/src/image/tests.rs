use super::Image;
use crate::color::Color;
use crate::geometry::Rect;
use crate::surface::Canvas;

fn checker(size: u32) -> Image {
    let pixels = (0..size * size)
        .map(|index| {
            let (x, y) = (index % size, index / size);
            if (x + y) % 2 == 0 {
                Color::WHITE
            } else {
                Color::BLACK
            }
        })
        .collect();
    Image::from_pixels(size, size, pixels).expect("checker builds")
}

#[test]
fn bgra_bytes_round_trip() {
    let source = checker(4);
    let bytes = source.to_bgra_bytes();
    let restored = Image::from_bgra_bytes(4, 4, &bytes).expect("restores");
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(restored.pixel(x, y), source.pixel(x, y));
        }
    }
}

#[test]
fn a_wrongly_sized_byte_buffer_is_rejected() {
    assert!(Image::from_bgra_bytes(4, 4, &[0; 8]).is_none());
    assert!(Image::from_pixels(2, 2, vec![Color::WHITE; 3]).is_none());
}

#[test]
fn reducing_a_checkerboard_averages_rather_than_aliasing() {
    // Point sampling would return pure black or pure white; a box filter must
    // land near the mean.
    let reduced = checker(64).resized(1, 1);
    let grey = reduced.pixel(0, 0);
    assert!(
        (i16::from(grey.red) - 128).abs() <= 4,
        "expected the mean, got {grey:?}"
    );
}

#[test]
fn resizing_to_the_same_size_is_lossless() {
    let source = checker(8);
    let same = source.resized(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(same.pixel(x, y), source.pixel(x, y));
        }
    }
}

#[test]
fn filtering_does_not_bleed_colour_out_of_transparent_pixels() {
    // A red pixel beside a fully transparent one. Averaging straight alpha
    // would drag the result toward black; premultiplied averaging must keep the
    // hue and only reduce the alpha.
    let pixels = vec![
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 0, 0, 0),
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 0, 0, 0),
    ];
    let image = Image::from_pixels(2, 2, pixels).expect("builds");
    let reduced = image.resized(1, 1);
    let blended = reduced.pixel(0, 0);
    assert_eq!(blended.red, 255, "hue must survive: {blended:?}");
    assert_eq!(blended.green, 0);
    assert!(
        (i16::from(blended.alpha) - 128).abs() <= 2,
        "alpha should halve: {blended:?}"
    );
}

#[test]
fn opaque_bounds_finds_the_artwork_inside_padding() {
    let mut pixels = vec![Color::TRANSPARENT; 100];
    pixels[3 * 10 + 2] = Color::WHITE;
    pixels[6 * 10 + 7] = Color::WHITE;
    let image = Image::from_pixels(10, 10, pixels).expect("builds");
    assert_eq!(image.opaque_bounds(0), Some((2, 3, 8, 7)));
}

#[test]
fn opaque_bounds_of_an_empty_image_is_none() {
    let image = Image::from_pixels(4, 4, vec![Color::TRANSPARENT; 16]).expect("builds");
    assert!(image.opaque_bounds(0).is_none());
}

#[test]
fn cropping_extracts_the_requested_region() {
    let source = checker(8);
    let cropped = source.cropped(2, 3, 4, 4);
    assert_eq!(cropped.width(), 4);
    assert_eq!(cropped.pixel(0, 0), source.pixel(2, 3));
    assert_eq!(cropped.pixel(3, 3), source.pixel(5, 6));
}

#[test]
fn drawing_an_image_fills_its_destination() {
    let image = Image::from_pixels(2, 2, vec![Color::rgb(200, 60, 240); 4]).expect("builds");
    let mut canvas = Canvas::new(32, 32);
    canvas.clear(Color::BLACK);
    canvas.draw_image(&image, Rect::new(8.0, 8.0, 24.0, 24.0), 1.0);
    assert_eq!(canvas.pixel(16, 16), Color::rgb(200, 60, 240));
    assert_eq!(canvas.pixel(4, 4), Color::BLACK, "outside the destination");
}

#[test]
fn opacity_fades_a_drawn_image() {
    let image = Image::from_pixels(1, 1, vec![Color::WHITE]).expect("builds");
    let mut canvas = Canvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.draw_image(&image, canvas.bounds(), 0.5);
    let faded = canvas.pixel(4, 4);
    assert!((i16::from(faded.red) - 128).abs() <= 2, "got {faded:?}");
}

#[test]
fn a_transparent_image_leaves_the_canvas_alone() {
    let image = Image::from_pixels(2, 2, vec![Color::TRANSPARENT; 4]).expect("builds");
    let mut canvas = Canvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.draw_image(&image, canvas.bounds(), 1.0);
    assert_eq!(canvas.pixel(4, 4), Color::BLACK);
}

#[test]
fn drawing_outside_the_canvas_is_clipped() {
    let image = checker(4);
    let mut canvas = Canvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.draw_image(&image, Rect::new(-100.0, -100.0, -50.0, -50.0), 1.0);
    canvas.draw_image(&image, Rect::new(400.0, 400.0, 500.0, 500.0), 1.0);
    assert_eq!(canvas.pixel(4, 4), Color::BLACK);
}
