//! Colour sources: flat fills and multi-stop gradients.

use crate::color::Color;
use crate::geometry::Point;

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
                let axis = start.to(*end);
                let length_squared = axis.dot(axis);
                let position = if length_squared <= f32::EPSILON {
                    0.0
                } else {
                    start.to(at).dot(axis) / length_squared
                };
                sample_stops(stops, position)
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
}
