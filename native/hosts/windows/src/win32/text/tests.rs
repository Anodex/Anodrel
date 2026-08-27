//! Rendering checks kept beside the text module's observable behavior.

use super::{Align, TextSpec, draw, line_height, width};
use anodrel_canvas::{Canvas, Color, Paint, point};

#[test]
fn a_wider_run_measures_wider() {
    let short = TextSpec::new("A", 20, 400);
    let long = TextSpec::new("AAAAAAAAAA", 20, 400);
    assert!(width(&long) > width(&short));
}

#[test]
fn tracking_widens_a_run() {
    let plain = TextSpec::new("ANODREL", 20, 700);
    let tracked = TextSpec::new("ANODREL", 20, 700).tracked(6);
    assert!(width(&tracked) > width(&plain));
}

#[test]
fn a_larger_size_raises_the_line_height() {
    assert!(
        line_height(&TextSpec::new("Ag", 40, 400)) > line_height(&TextSpec::new("Ag", 12, 400))
    );
}

#[test]
fn measuring_agrees_with_the_rasterized_run() {
    let spec = TextSpec::new("Anodrel Sample", 24, 500);
    let measured = width(&spec);
    let mut canvas = Canvas::new(400, 64);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        &spec,
        point(10.0, 10.0),
        Align::Left,
        &Paint::solid(Color::WHITE),
    );
    let last_lit = (0..400)
        .rev()
        .find(|x| (0..64).any(|y| canvas.pixel(*x, y).red > 40))
        .expect("the run drew");
    assert!(
        (last_lit as f32) <= 10.0 + measured + 2.0,
        "drawn run at {last_lit} exceeds its measured advance {measured}"
    );
}

#[test]
fn an_empty_run_measures_zero_and_draws_nothing() {
    let empty = TextSpec::new("", 20, 400);
    assert_eq!(width(&empty), 0.0);
    let mut canvas = Canvas::new(32, 32);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        &empty,
        point(4.0, 4.0),
        Align::Left,
        &Paint::solid(Color::WHITE),
    );
    assert_eq!(canvas.pixel(8, 8), Color::BLACK);
}

#[test]
fn drawing_marks_the_canvas() {
    let spec = TextSpec::new("Anodrel", 24, 600);
    let mut canvas = Canvas::new(240, 64);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        &spec,
        point(8.0, 8.0),
        Align::Left,
        &Paint::solid(Color::WHITE),
    );
    let lit = (0..64)
        .flat_map(|y| (0..240).map(move |x| (x, y)))
        .filter(|(x, y)| canvas.pixel(*x, *y).red > 40)
        .count();
    assert!(lit > 30, "expected glyph coverage, found {lit} lit pixels");
}

#[test]
fn alignment_moves_a_run_relative_to_its_anchor() {
    let spec = TextSpec::new("Anodrel", 24, 600);
    let run_width = width(&spec);
    assert!(run_width > 0.0);
    let first_lit_column = |align| {
        let mut canvas = Canvas::new(400, 48);
        canvas.clear(Color::BLACK);
        draw(
            &mut canvas,
            &spec,
            point(200.0, 8.0),
            align,
            &Paint::solid(Color::WHITE),
        );
        (0..400).find(|x| (0..48).any(|y| canvas.pixel(*x, y).red > 40))
    };

    let left = first_lit_column(Align::Left).expect("left aligned run drew");
    let center = first_lit_column(Align::Center).expect("centred run drew");
    let right = first_lit_column(Align::Right).expect("right aligned run drew");
    assert!(right < center && center < left);
}

#[test]
fn a_gradient_paint_varies_across_a_run() {
    let spec = TextSpec::new("ANODRELANODREL", 32, 700);
    let mut canvas = Canvas::new(400, 64);
    canvas.clear(Color::BLACK);
    draw(
        &mut canvas,
        &spec,
        point(4.0, 8.0),
        Align::Left,
        &Paint::horizontal(
            4.0,
            4.0 + width(&spec),
            Color::hex(0xFF0000),
            Color::hex(0x0000FF),
        ),
    );
    let count = |channel: fn(&anodrel_canvas::Color) -> u8| {
        (0..64)
            .flat_map(|y| (0..400).map(move |x| (x, y)))
            .map(|(x, y)| canvas.pixel(x, y))
            .filter(|color| channel(color) > 80)
            .count()
    };
    assert!(
        count(|color| color.red) > 0 && count(|color| color.blue) > 0,
        "gradient text should span both ends"
    );
}
