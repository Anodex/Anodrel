//! Closed polygonal paths, the only geometry the rasterizer understands.
//!
//! Curves are flattened by the builders rather than carried through the
//! pipeline. Keeping one primitive means the fill rule, the offset routine, and
//! the coverage rasterizer each have a single code path to get right.

use crate::geometry::{Point, Rect};

/// A shape made of one or more implicitly closed contours.
///
/// Multiple contours combine under the non-zero winding rule, so a hole is a
/// contour wound opposite to the one enclosing it.
#[derive(Clone, Debug, Default)]
pub struct Path {
    contours: Vec<Vec<Point>>,
}

impl Path {
    /// Builds an empty path.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            contours: Vec::new(),
        }
    }

    /// Builds a path from a single closed contour.
    ///
    /// The closing edge is implied; do not repeat the first point.
    #[must_use]
    pub fn polygon(points: impl IntoIterator<Item = Point>) -> Self {
        let mut path = Self::new();
        path.push_contour(points);
        path
    }

    /// Builds a rectangle.
    #[must_use]
    pub fn rect(rect: Rect) -> Self {
        Self::polygon([
            Point::new(rect.left, rect.top),
            Point::new(rect.right, rect.top),
            Point::new(rect.right, rect.bottom),
            Point::new(rect.left, rect.bottom),
        ])
    }

    /// Builds a rectangle with equal corner radii.
    ///
    /// The radius is clamped to half of the shorter side, so an over-large
    /// radius degrades to a stadium instead of self-intersecting.
    #[must_use]
    pub fn rounded_rect(rect: Rect, radius: f32) -> Self {
        let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
        if radius <= 0.0 {
            return Self::rect(rect);
        }
        let steps = corner_steps(radius);
        let mut points = Vec::with_capacity(steps * 4 + 4);
        let corners = [
            // centre, start angle (radians, clockwise on screen)
            (
                Point::new(rect.right - radius, rect.top + radius),
                -std::f32::consts::FRAC_PI_2,
            ),
            (Point::new(rect.right - radius, rect.bottom - radius), 0.0),
            (
                Point::new(rect.left + radius, rect.bottom - radius),
                std::f32::consts::FRAC_PI_2,
            ),
            (
                Point::new(rect.left + radius, rect.top + radius),
                std::f32::consts::PI,
            ),
        ];
        for (center, start) in corners {
            for step in 0..=steps {
                let angle = start + std::f32::consts::FRAC_PI_2 * (step as f32) / (steps as f32);
                points.push(Point::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                ));
            }
        }
        Self::polygon(points)
    }

    /// Builds a circle.
    #[must_use]
    pub fn circle(center: Point, radius: f32) -> Self {
        Self::ellipse(Rect::centered(center, radius * 2.0, radius * 2.0))
    }

    /// Builds an ellipse inscribed in `rect`.
    #[must_use]
    pub fn ellipse(rect: Rect) -> Self {
        let center = rect.center();
        let radius_x = rect.width() / 2.0;
        let radius_y = rect.height() / 2.0;
        let steps = (corner_steps(radius_x.max(radius_y)) * 4).max(16);
        let points = (0..steps).map(|step| {
            let angle = std::f32::consts::TAU * (step as f32) / (steps as f32);
            Point::new(
                center.x + radius_x * angle.cos(),
                center.y + radius_y * angle.sin(),
            )
        });
        Self::polygon(points)
    }

    /// Builds an annulus, used for rings and circular progress tracks.
    ///
    /// The inner contour is wound backwards so the non-zero rule leaves a hole.
    #[must_use]
    pub fn ring(center: Point, outer_radius: f32, thickness: f32) -> Self {
        let inner_radius = (outer_radius - thickness).max(0.0);
        let mut path = Self::circle(center, outer_radius);
        if inner_radius > 0.0 {
            let mut inner: Vec<Point> = Self::circle(center, inner_radius).contours[0].clone();
            inner.reverse();
            path.push_contour(inner);
        }
        path
    }

    /// Appends a closed contour. Contours with fewer than three points are ignored.
    pub fn push_contour(&mut self, points: impl IntoIterator<Item = Point>) {
        self.push_owned_contour(points.into_iter().collect());
    }

    /// Appends an already-owned closed contour without copying its points.
    ///
    /// Contours with fewer than three points are ignored. The closing edge is
    /// implied, so callers must not repeat the first point at the end.
    pub fn push_owned_contour(&mut self, points: Vec<Point>) {
        if points.len() >= 3 {
            self.contours.push(points);
        }
    }

    /// Returns the contours making up the path.
    #[must_use]
    pub fn contours(&self) -> &[Vec<Point>] {
        &self.contours
    }

    /// Returns `true` when the path encloses nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.contours.is_empty()
    }

    /// Returns the tight bounding box, or an empty rectangle for an empty path.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let mut bounds: Option<Rect> = None;
        for contour in &self.contours {
            for vertex in contour {
                bounds = Some(match bounds {
                    None => Rect::new(vertex.x, vertex.y, vertex.x, vertex.y),
                    Some(current) => Rect::new(
                        current.left.min(vertex.x),
                        current.top.min(vertex.y),
                        current.right.max(vertex.x),
                        current.bottom.max(vertex.y),
                    ),
                });
            }
        }
        bounds.unwrap_or_default()
    }

    /// Returns the path translated by `dx` and `dy`.
    #[must_use]
    pub fn translate(&self, dx: f32, dy: f32) -> Self {
        self.map_points(|vertex| vertex.offset(dx, dy))
    }

    /// Returns the path scaled about `origin`.
    #[must_use]
    pub fn scale_about(&self, origin: Point, factor: f32) -> Self {
        self.map_points(|vertex| {
            Point::new(
                origin.x + (vertex.x - origin.x) * factor,
                origin.y + (vertex.y - origin.y) * factor,
            )
        })
    }

    /// Returns the path mapped from the unit square into `rect`.
    ///
    /// Brand geometry is authored in normalised `0.0..=1.0` space; this is how
    /// it is placed at a concrete size without duplicating the coordinates.
    #[must_use]
    pub fn fit_unit_square(&self, rect: Rect) -> Self {
        let width = rect.width();
        let height = rect.height();
        self.map_points(|vertex| {
            Point::new(rect.left + vertex.x * width, rect.top + vertex.y * height)
        })
    }

    /// Returns a copy moved `distance` pixels toward the shape's interior.
    ///
    /// Adjacent offset edges are intersected, so corners keep a mitre instead of
    /// splaying open. Nearly parallel edges fall back to a plain normal offset,
    /// which avoids the spike a raw intersection would produce. The result is
    /// the inner boundary of a bevel band; pairing each original edge with its
    /// offset counterpart yields quads that tile the band without overlapping.
    #[must_use]
    pub fn inset(&self, distance: f32) -> Self {
        if distance == 0.0 {
            return self.clone();
        }
        let mut result = Self::new();
        for contour in &self.contours {
            let count = contour.len();
            let orientation = if signed_area(contour) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let mut offset = Vec::with_capacity(count);
            for index in 0..count {
                let previous = contour[(index + count - 1) % count];
                let current = contour[index];
                let next = contour[(index + 1) % count];
                offset.push(offset_corner(
                    previous,
                    current,
                    next,
                    distance,
                    orientation,
                ));
            }
            result.push_contour(offset);
        }
        result
    }

    /// Returns the quads that tile the band between `self` and `self.inset(distance)`.
    ///
    /// Each quad is paired with the outward unit normal of the edge it came
    /// from, which is what a bevel needs to decide whether an edge faces the
    /// light. Quads meet exactly at the mitred corners, so they can be filled
    /// with translucent paint without seams or double-blended overlaps.
    #[must_use]
    pub fn bevel_bands(&self, distance: f32) -> Vec<BevelBand> {
        let inner = self.inset(distance);
        let mut bands = Vec::new();
        for (contour, inner_contour) in self.contours.iter().zip(inner.contours.iter()) {
            let count = contour.len();
            if inner_contour.len() != count {
                continue;
            }
            let orientation = if signed_area(contour) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            for index in 0..count {
                let next = (index + 1) % count;
                let Some(normal) = edge_inward_normal(contour[index], contour[next], orientation)
                else {
                    continue;
                };
                bands.push(BevelBand {
                    quad: Path::polygon([
                        contour[index],
                        contour[next],
                        inner_contour[next],
                        inner_contour[index],
                    ]),
                    outward_normal: normal.scale(-1.0),
                });
            }
        }
        bands
    }

    fn map_points(&self, mut transform: impl FnMut(Point) -> Point) -> Self {
        Self {
            contours: self
                .contours
                .iter()
                .map(|contour| contour.iter().map(|vertex| transform(*vertex)).collect())
                .collect(),
        }
    }
}

/// One edge of a bevel: the band quad and the direction that edge faces.
#[derive(Clone, Debug)]
pub struct BevelBand {
    /// The four-sided region between the outer edge and its inset counterpart.
    pub quad: Path,
    /// Unit normal pointing away from the shape's interior.
    pub outward_normal: Point,
}

fn corner_steps(radius: f32) -> usize {
    ((radius * 0.75) as usize).clamp(4, 24)
}

fn signed_area(contour: &[Point]) -> f32 {
    let count = contour.len();
    let mut total = 0.0;
    for index in 0..count {
        let current = contour[index];
        let next = contour[(index + 1) % count];
        total += current.x * next.y - next.x * current.y;
    }
    total / 2.0
}

/// Returns the unit normal of edge `from -> to` that points into the shape.
fn edge_inward_normal(from: Point, to: Point, orientation: f32) -> Option<Point> {
    let direction = from.to(to).normalized()?;
    Some(Point::new(-direction.y, direction.x).scale(orientation))
}

fn offset_corner(
    previous: Point,
    current: Point,
    next: Point,
    distance: f32,
    orientation: f32,
) -> Point {
    let incoming = edge_inward_normal(previous, current, orientation);
    let outgoing = edge_inward_normal(current, next, orientation);
    let (incoming, outgoing) = match (incoming, outgoing) {
        (Some(incoming), Some(outgoing)) => (incoming, outgoing),
        (Some(only), None) | (None, Some(only)) => {
            return current.offset(only.x * distance, only.y * distance);
        }
        (None, None) => return current,
    };

    let first_direction = previous.to(current);
    let second_direction = current.to(next);
    let cross = first_direction.x * second_direction.y - first_direction.y * second_direction.x;
    if cross.abs() <= 1e-4 {
        // Collinear edges share a normal, so the plain offset is already exact.
        return current.offset(incoming.x * distance, incoming.y * distance);
    }

    let first_origin = previous.offset(incoming.x * distance, incoming.y * distance);
    let second_origin = current.offset(outgoing.x * distance, outgoing.y * distance);
    let between = first_origin.to(second_origin);
    let travel = (between.x * second_direction.y - between.y * second_direction.x) / cross;
    let corner = Point::new(
        first_origin.x + first_direction.x * travel,
        first_origin.y + first_direction.y * travel,
    );

    // A very sharp corner drives the mitre far from the shape. Clamping keeps
    // the band well formed at the cost of a slightly blunted tip.
    let limit = distance.abs() * 6.0;
    if current.to(corner).length() > limit {
        current.offset(incoming.x * distance, incoming.y * distance)
    } else {
        corner
    }
}

#[cfg(test)]
mod tests;
