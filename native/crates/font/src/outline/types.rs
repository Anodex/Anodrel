//! Public, owned simple-outline values.

/// One signed design-unit rectangle from a TrueType glyph header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphBounds {
    x_min: i16,
    y_min: i16,
    x_max: i16,
    y_max: i16,
}

impl GlyphBounds {
    /// Returns the least horizontal design-unit coordinate.
    pub const fn x_min(self) -> i16 {
        self.x_min
    }

    /// Returns the least vertical design-unit coordinate.
    pub const fn y_min(self) -> i16 {
        self.y_min
    }

    /// Returns the greatest horizontal design-unit coordinate.
    pub const fn x_max(self) -> i16 {
        self.x_max
    }

    /// Returns the greatest vertical design-unit coordinate.
    pub const fn y_max(self) -> i16 {
        self.y_max
    }

    pub(crate) fn new(x_min: i16, y_min: i16, x_max: i16, y_max: i16) -> Option<Self> {
        (x_min <= x_max && y_min <= y_max).then_some(Self {
            x_min,
            y_min,
            x_max,
            y_max,
        })
    }

    pub(crate) const fn empty() -> Self {
        Self {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        }
    }
}

/// One TrueType simple-glyph point in signed font design units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphPoint {
    x: i16,
    y: i16,
    on_curve: bool,
}

impl GlyphPoint {
    /// Returns the horizontal design-unit coordinate.
    pub const fn x(self) -> i16 {
        self.x
    }

    /// Returns the vertical design-unit coordinate.
    pub const fn y(self) -> i16 {
        self.y
    }

    /// Returns whether this point lies on the glyph's quadratic curve.
    pub const fn is_on_curve(self) -> bool {
        self.on_curve
    }

    pub(crate) const fn new(x: i16, y: i16, on_curve: bool) -> Self {
        Self { x, y, on_curve }
    }

    pub(crate) fn set_y(&mut self, y: i16) {
        self.y = y;
    }
}

/// One owned, validated simple TrueType glyph outline.
#[derive(Debug, Eq, PartialEq)]
pub struct GlyphOutline {
    bounds: GlyphBounds,
    points: Vec<GlyphPoint>,
    contour_ends: Vec<usize>,
}

impl GlyphOutline {
    /// Returns the glyph header's design-unit bounds.
    pub const fn bounds(&self) -> GlyphBounds {
        self.bounds
    }

    /// Returns the number of simple contours.
    pub fn contour_count(&self) -> usize {
        self.contour_ends.len()
    }

    /// Returns the total number of points across every contour.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Returns one complete contour without copying its points.
    pub fn point_slice(&self, contour: usize) -> Option<&[GlyphPoint]> {
        let end = *self.contour_ends.get(contour)?;
        let start = contour
            .checked_sub(1)
            .and_then(|previous| self.contour_ends.get(previous).copied())
            .map_or(0, |previous_end| previous_end + 1);
        self.points.get(start..=end)
    }

    pub(crate) fn new(
        bounds: GlyphBounds,
        points: Vec<GlyphPoint>,
        contour_ends: Vec<usize>,
    ) -> Self {
        Self {
            bounds,
            points,
            contour_ends,
        }
    }

    pub(crate) fn empty() -> Self {
        Self::new(GlyphBounds::empty(), Vec::new(), Vec::new())
    }
}
