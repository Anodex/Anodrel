//! Colour sources: flat fills and multi-stop gradients.

use crate::color::Color;
use crate::geometry::Point;

/// Precomputed colours held by one explicit quantized linear gradient.
const QUANTIZED_LINEAR_SAMPLES: usize = 512;

/// One colour anchored at a position along a gradient axis.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    /// Position along the axis, clamped to `0.0..=1.0`.
    pub position: f32,
    /// Colour at that position.
    pub color: Color,
}

impl Stop {
    /// Builds a gradient stop.
    #[must_use]
    pub const fn new(position: f32, color: Color) -> Self {
        Self { position, color }
    }
}

/// Shorthand for [`Stop::new`].
#[must_use]
pub const fn stop(position: f32, color: Color) -> Stop {
    Stop::new(position, color)
}

/// A source of colour evaluated per pixel.
///
/// Paints are pure functions of position, so the rasterizer can sample them at
/// pixel centres without tracking any state between spans.
#[derive(Clone, Debug)]
pub enum Paint {
    /// A single colour everywhere.
    Solid(Color),
    /// A gradient interpolated along the axis from `start` to `end`.
    ///
    /// Positions before the first stop and after the last clamp to the end
    /// colours rather than repeating.
    Linear {
        /// Axis origin, mapping to position `0.0`.
        start: Point,
        /// Axis end, mapping to position `1.0`.
        end: Point,
        /// Stops in ascending position order.
        stops: Vec<Stop>,
    },
    /// A bounded approximate linear gradient for a diffuse visual effect.
    ///
    /// The colour ramp contains [`QUANTIZED_LINEAR_SAMPLES`] samples and is
    /// selected by nearest position. Exact linear gradients remain the default
    /// for callers that carry semantic content.
    LinearQuantized {
        /// Axis origin, mapping to ramp position `0.0`.
        start: Point,
        /// Axis end, mapping to ramp position `1.0`.
        end: Point,
        /// Precomputed colours from the caller's exact stop table.
        ramp: Vec<Color>,
    },
    /// A gradient interpolated by distance from `center`.
    Radial {
        /// Centre, mapping to position `0.0`.
        center: Point,
        /// Distance mapping to position `1.0`.
        radius: f32,
        /// Stops in ascending position order.
        stops: Vec<Stop>,
    },
}

impl Paint {
    /// Builds a flat fill.
    #[must_use]
    pub const fn solid(color: Color) -> Self {
        Self::Solid(color)
    }

    /// Builds a linear gradient.
    #[must_use]
    pub fn linear(start: Point, end: Point, stops: impl Into<Vec<Stop>>) -> Self {
        Self::Linear {
            start,
            end,
            stops: stops.into(),
        }
    }

    /// Builds a bounded quantized linear gradient from exact colour stops.
    ///
    /// This replaces per-pixel stop lookup and interpolation with a nearest
    /// lookup in 512 colours. Use it only where a caller has independently
    /// bounded and tested the resulting visual error; see Decision 0176.
    #[must_use]
    pub fn linear_quantized(start: Point, end: Point, stops: impl Into<Vec<Stop>>) -> Self {
        let stops = stops.into();
        let last = (QUANTIZED_LINEAR_SAMPLES - 1) as f32;
        let mut ramp = Vec::with_capacity(QUANTIZED_LINEAR_SAMPLES);
        for index in 0..QUANTIZED_LINEAR_SAMPLES {
            ramp.push(sample_stops(&stops, index as f32 / last));
        }
        Self::LinearQuantized { start, end, ramp }
    }

    /// Builds a vertical linear gradient between two colours.
    #[must_use]
    pub fn vertical(top: f32, bottom: f32, from: Color, to: Color) -> Self {
        Self::linear(
            Point::new(0.0, top),
            Point::new(0.0, bottom),
            vec![stop(0.0, from), stop(1.0, to)],
        )
    }

    /// Builds a horizontal linear gradient between two colours.
    #[must_use]
    pub fn horizontal(left: f32, right: f32, from: Color, to: Color) -> Self {
        Self::linear(
            Point::new(left, 0.0),
            Point::new(right, 0.0),
            vec![stop(0.0, from), stop(1.0, to)],
        )
    }

    /// Builds a radial gradient.
    #[must_use]
    pub fn radial(center: Point, radius: f32, stops: impl Into<Vec<Stop>>) -> Self {
        Self::Radial {
            center,
            radius,
            stops: stops.into(),
        }
    }

    /// Returns the colour at a point in canvas space.
    #[must_use]
    pub fn sample(&self, at: Point) -> Color {
        match self {
            Self::Solid(color) => *color,
            Self::Linear { start, end, stops } => {
                sample_stops(stops, linear_position(*start, *end, at))
            }
            Self::LinearQuantized { start, end, ramp } => {
                sample_ramp(ramp, linear_position(*start, *end, at))
            }
            Self::Radial {
                center,
                radius,
                stops,
            } => {
                let position = if *radius <= f32::EPSILON {
                    1.0
                } else {
                    center.to(at).length() / radius
                };
                sample_stops(stops, position)
            }
        }
    }

    /// Returns a copy with every stop's opacity scaled by `factor`.
    ///
    /// Reveal animations use this so a paint can fade without its stop table
    /// being rebuilt at the call site.
    #[must_use]
    pub fn scale_alpha(&self, factor: f32) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(color.scale_alpha(factor)),
            Self::Linear { start, end, stops } => Self::Linear {
                start: *start,
                end: *end,
                stops: scaled_stops(stops, factor),
            },
            Self::LinearQuantized { start, end, ramp } => Self::LinearQuantized {
                start: *start,
                end: *end,
                ramp: ramp.iter().map(|color| color.scale_alpha(factor)).collect(),
            },
            Self::Radial {
                center,
                radius,
                stops,
            } => Self::Radial {
                center: *center,
                radius: *radius,
                stops: scaled_stops(stops, factor),
            },
        }
    }
}

fn linear_position(start: Point, end: Point, at: Point) -> f32 {
    let axis = start.to(end);
    let length_squared = axis.dot(axis);
    if length_squared <= f32::EPSILON {
        0.0
    } else {
        start.to(at).dot(axis) / length_squared
    }
}

fn sample_ramp(ramp: &[Color], position: f32) -> Color {
    let Some(last_index) = ramp.len().checked_sub(1) else {
        return Color::TRANSPARENT;
    };
    let index = (position.clamp(0.0, 1.0) * last_index as f32).round() as usize;
    ramp[index]
}

fn scaled_stops(stops: &[Stop], factor: f32) -> Vec<Stop> {
    stops
        .iter()
        .map(|entry| Stop::new(entry.position, entry.color.scale_alpha(factor)))
        .collect()
}

fn sample_stops(stops: &[Stop], position: f32) -> Color {
    let Some(first) = stops.first() else {
        return Color::TRANSPARENT;
    };
    let last = stops.last().unwrap_or(first);
    if position <= first.position {
        return first.color;
    }
    if position >= last.position {
        return last.color;
    }
    for window in stops.windows(2) {
        let (from, to) = (window[0], window[1]);
        if position >= from.position && position <= to.position {
            let span = to.position - from.position;
            let amount = if span <= f32::EPSILON {
                0.0
            } else {
                (position - from.position) / span
            };
            return from.color.lerp(to.color, amount);
        }
    }
    last.color
}

#[cfg(test)]
mod tests {
    use super::{Paint, stop};
    use crate::color::Color;
    use crate::geometry::point;

    #[test]
    fn a_linear_gradient_clamps_outside_its_axis() {
        let paint = Paint::horizontal(0.0, 100.0, Color::BLACK, Color::WHITE);
        assert_eq!(paint.sample(point(-50.0, 0.0)), Color::BLACK);
        assert_eq!(paint.sample(point(150.0, 0.0)), Color::WHITE);
    }

    #[test]
    fn a_linear_gradient_interpolates_at_the_midpoint() {
        let paint = Paint::horizontal(0.0, 100.0, Color::BLACK, Color::WHITE);
        let middle = paint.sample(point(50.0, 0.0));
        assert!((i16::from(middle.red) - 128).abs() <= 1);
    }

    #[test]
    fn a_linear_gradient_ignores_the_perpendicular_axis() {
        let paint = Paint::horizontal(0.0, 100.0, Color::BLACK, Color::WHITE);
        assert_eq!(
            paint.sample(point(25.0, 0.0)),
            paint.sample(point(25.0, 900.0))
        );
    }

    #[test]
    fn a_radial_gradient_reaches_its_edge_colour_at_the_radius() {
        let paint = Paint::radial(
            point(10.0, 10.0),
            5.0,
            vec![stop(0.0, Color::WHITE), stop(1.0, Color::BLACK)],
        );
        assert_eq!(paint.sample(point(10.0, 10.0)), Color::WHITE);
        assert_eq!(paint.sample(point(15.0, 10.0)), Color::BLACK);
        assert_eq!(paint.sample(point(90.0, 10.0)), Color::BLACK);
    }

    #[test]
    fn a_degenerate_axis_does_not_divide_by_zero() {
        let paint = Paint::linear(
            point(4.0, 4.0),
            point(4.0, 4.0),
            vec![stop(0.0, Color::WHITE), stop(1.0, Color::BLACK)],
        );
        assert_eq!(paint.sample(point(80.0, 12.0)), Color::WHITE);
    }

    #[test]
    fn an_empty_stop_table_samples_transparent() {
        let paint = Paint::linear(point(0.0, 0.0), point(1.0, 0.0), Vec::new());
        assert_eq!(paint.sample(point(0.5, 0.0)), Color::TRANSPARENT);
    }

    #[test]
    fn scale_alpha_fades_every_stop() {
        let paint = Paint::horizontal(0.0, 10.0, Color::WHITE, Color::BLACK).scale_alpha(0.5);
        assert_eq!(paint.sample(point(0.0, 0.0)).alpha, 128);
        assert_eq!(paint.sample(point(10.0, 0.0)).alpha, 128);
    }

    #[test]
    fn a_quantized_linear_gradient_stays_within_one_channel_level() {
        let stops = vec![
            stop(0.0, Color::rgba(255, 3, 79, 211)),
            stop(0.43, Color::rgba(17, 249, 131, 19)),
            stop(1.0, Color::rgba(5, 47, 255, 233)),
        ];
        let exact = Paint::linear(point(13.4, 0.0), point(507.8, 0.0), stops.clone());
        let quantized = Paint::linear_quantized(point(13.4, 0.0), point(507.8, 0.0), stops);

        for index in 0..=16_384 {
            let x = -20.0 + index as f32 * 560.0 / 16_384.0;
            let (a, b) = (
                exact.sample(point(x, 12.0)),
                quantized.sample(point(x, 12.0)),
            );
            for (exact, approximate) in [
                (a.red, b.red),
                (a.green, b.green),
                (a.blue, b.blue),
                (a.alpha, b.alpha),
            ] {
                assert!(
                    (i16::from(exact) - i16::from(approximate)).abs() <= 1,
                    "sample at {x} differs by more than one channel level"
                );
            }
        }
    }
}
