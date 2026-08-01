//! Geometry shared by layout and host adapters.

use crate::Insets;

/// A point in logical pixels, with `y` increasing downward.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiPoint {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
}

impl UiPoint {
    /// Builds a point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and height in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiSize {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl UiSize {
    /// Builds a size.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub(crate) fn sanitized(self) -> Self {
        Self::new(sanitize_extent(self.width), sanitize_extent(self.height))
    }
}

/// An axis-aligned logical-pixel rectangle.
///
/// Far edges are exclusive for hit testing. A rectangle with non-finite edges
/// or non-positive dimensions is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiRect {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge, exclusive.
    pub right: f32,
    /// Bottom edge, exclusive.
    pub bottom: f32,
}

impl UiRect {
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

    /// Returns whether this rectangle contains no visible logical-pixel area.
    #[must_use]
    pub fn is_empty(self) -> bool {
        !self.left.is_finite()
            || !self.top.is_finite()
            || !self.right.is_finite()
            || !self.bottom.is_finite()
            || self.right <= self.left
            || self.bottom <= self.top
    }

    /// Returns the overlap with `other`, which can be empty.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    /// Returns this rectangle with its edges moved inward by the given padding.
    #[must_use]
    pub(crate) fn inset(self, padding: Insets) -> Self {
        Self::new(
            self.left + f32::from(padding.left),
            self.top + f32::from(padding.top),
            self.right - f32::from(padding.right),
            self.bottom - f32::from(padding.bottom),
        )
    }

    /// Returns whether a finite point is inside this rectangle.
    #[must_use]
    pub fn contains(self, point: UiPoint) -> bool {
        !self.is_empty()
            && point.x.is_finite()
            && point.y.is_finite()
            && point.x >= self.left
            && point.x < self.right
            && point.y >= self.top
            && point.y < self.bottom
    }
}

fn sanitize_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
