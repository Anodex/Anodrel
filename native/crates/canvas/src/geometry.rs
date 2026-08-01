//! Floating-point points and rectangles shared by paths, paints, and layout.

/// A point in canvas space, measured in pixels with `y` increasing downward.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Point {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
}

/// Shorthand for [`Point::new`].
#[must_use]
pub const fn point(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

impl Point {
    /// Builds a point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the component-wise sum.
    #[must_use]
    pub fn offset(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    /// Returns the vector from `self` to `other`.
    #[must_use]
    pub fn to(self, other: Self) -> Self {
        Self::new(other.x - self.x, other.y - self.y)
    }

    /// Returns the Euclidean length, treating the point as a vector.
    #[must_use]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    /// Returns a unit-length vector, or `None` when the length is degenerate.
    #[must_use]
    pub fn normalized(self) -> Option<Self> {
        let length = self.length();
        if length <= f32::EPSILON {
            None
        } else {
            Some(Self::new(self.x / length, self.y / length))
        }
    }

    /// Returns the dot product with `other`.
    #[must_use]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Scales both components.
    #[must_use]
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    /// Linearly interpolates toward `other`.
    #[must_use]
    pub fn lerp(self, other: Self, amount: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * amount,
            self.y + (other.y - self.y) * amount,
        )
    }
}

/// An axis-aligned rectangle.
///
/// A rectangle is empty when `right <= left` or `bottom <= top`; the rasterizer
/// treats empty rectangles as no-ops rather than errors.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge, exclusive.
    pub right: f32,
    /// Bottom edge, exclusive.
    pub bottom: f32,
}

impl Rect {
    /// Builds a rectangle from its edges.
    #[must_use]
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Builds a rectangle from an origin and a size.
    #[must_use]
    pub fn from_size(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self::new(left, top, left + width, top + height)
    }

    /// Builds a rectangle centred on a point.
    #[must_use]
    pub fn centered(center: Point, width: f32, height: f32) -> Self {
        Self::new(
            center.x - width / 2.0,
            center.y - height / 2.0,
            center.x + width / 2.0,
            center.y + height / 2.0,
        )
    }

    /// Returns the width, which may be negative for an empty rectangle.
    #[must_use]
    pub fn width(self) -> f32 {
        self.right - self.left
    }

    /// Returns the height, which may be negative for an empty rectangle.
    #[must_use]
    pub fn height(self) -> f32 {
        self.bottom - self.top
    }

    /// Returns the centre point.
    #[must_use]
    pub fn center(self) -> Point {
        Point::new(
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
        )
    }

    /// Returns `true` when the rectangle encloses no pixels.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// Grows the rectangle by `amount` on every side. Negative values shrink it.
    #[must_use]
    pub fn inflate(self, amount: f32) -> Self {
        Self::new(
            self.left - amount,
            self.top - amount,
            self.right + amount,
            self.bottom + amount,
        )
    }

    /// Moves the rectangle without changing its size.
    #[must_use]
    pub fn translate(self, dx: f32, dy: f32) -> Self {
        Self::new(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
        )
    }

    /// Returns the overlap with `other`, which may be empty.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    /// Returns the smallest rectangle containing both inputs.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self::new(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Returns `true` when `point` lies inside the rectangle.
    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::{Point, Rect, point};

    #[test]
    fn normalized_rejects_a_degenerate_vector() {
        assert!(point(0.0, 0.0).normalized().is_none());
        let unit = point(3.0, 4.0).normalized().expect("vector has length");
        assert!((unit.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn intersection_of_disjoint_rectangles_is_empty() {
        let left = Rect::new(0.0, 0.0, 10.0, 10.0);
        let right = Rect::new(20.0, 20.0, 30.0, 30.0);
        assert!(left.intersect(right).is_empty());
        assert!(!left.intersect(Rect::new(5.0, 5.0, 15.0, 15.0)).is_empty());
    }

    #[test]
    fn union_ignores_empty_inputs() {
        let real = Rect::new(2.0, 3.0, 8.0, 9.0);
        assert_eq!(Rect::default().union(real), real);
        assert_eq!(real.union(Rect::default()), real);
    }

    #[test]
    fn contains_excludes_the_far_edges() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains(Point::new(0.0, 0.0)));
        assert!(!rect.contains(Point::new(10.0, 5.0)));
        assert!(!rect.contains(Point::new(5.0, 10.0)));
    }

    #[test]
    fn centered_rectangle_keeps_its_center() {
        let rect = Rect::centered(Point::new(50.0, 20.0), 10.0, 4.0);
        assert_eq!(rect.center(), Point::new(50.0, 20.0));
        assert_eq!(rect.width(), 10.0);
        assert_eq!(rect.height(), 4.0);
    }
}
