//! Exact, renderer-neutral quadratic path values.

/// One exact path coordinate stored in doubled font design units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphPathPoint {
    x_twice: i32,
    y_twice: i32,
}

impl GlyphPathPoint {
    /// Returns the horizontal coordinate in doubled design units.
    pub const fn x_twice(self) -> i32 {
        self.x_twice
    }

    /// Returns the vertical coordinate in doubled design units.
    pub const fn y_twice(self) -> i32 {
        self.y_twice
    }

    pub(crate) const fn new(x_twice: i32, y_twice: i32) -> Self {
        Self { x_twice, y_twice }
    }
}

/// One closed-contour segment ending at an exact path point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphSegment {
    /// A straight segment from the prior endpoint to `to`.
    LineTo {
        /// The exact endpoint in doubled design units.
        to: GlyphPathPoint,
    },
    /// A quadratic Bézier segment from the prior endpoint through `control` to `to`.
    QuadraticTo {
        /// The exact off-curve control point in doubled design units.
        control: GlyphPathPoint,
        /// The exact endpoint in doubled design units.
        to: GlyphPathPoint,
    },
}

/// One owned, closed sequence of exact line and quadratic glyph segments.
#[derive(Debug, Eq, PartialEq)]
pub struct GlyphPath {
    contour_starts: Vec<GlyphPathPoint>,
    contour_segment_ends: Vec<usize>,
    segments: Vec<GlyphSegment>,
}

impl GlyphPath {
    /// Returns the number of closed contours in this path.
    pub fn contour_count(&self) -> usize {
        self.contour_starts.len()
    }

    /// Returns the total number of line and quadratic segments.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Returns one contour's start point.
    pub fn contour_start(&self, contour: usize) -> Option<GlyphPathPoint> {
        self.contour_starts.get(contour).copied()
    }

    /// Returns one complete closed contour's ordered segments without copying them.
    pub fn segment_slice(&self, contour: usize) -> Option<&[GlyphSegment]> {
        let end = *self.contour_segment_ends.get(contour)?;
        let start = contour
            .checked_sub(1)
            .and_then(|previous| self.contour_segment_ends.get(previous).copied())
            .unwrap_or(0);
        self.segments.get(start..end)
    }

    pub(crate) fn new(
        contour_starts: Vec<GlyphPathPoint>,
        contour_segment_ends: Vec<usize>,
        segments: Vec<GlyphSegment>,
    ) -> Self {
        Self {
            contour_starts,
            contour_segment_ends,
            segments,
        }
    }
}
