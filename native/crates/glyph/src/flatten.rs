//! Device-space adaptive flattening of one owned quadratic glyph path.

use anodrel_canvas::{Path, Point};
use anodrel_font::{GlyphPath, GlyphSegment};

use crate::{GlyphPlacement, GlyphRenderError};

const MAX_SUBDIVISION_DEPTH: u8 = 8;
const SUBDIVISION_STACK_CAPACITY: usize = MAX_SUBDIVISION_DEPTH as usize + 1;
const MAX_GLYPH_VERTICES: usize = 65_536;
const MAX_DEVIATION_SQUARED: f32 = 1.0 / 16.0;

/// Converts a validated quadratic glyph path into closed canvas polygon contours.
///
/// The supplied placement is the only coordinate-system bridge. Curves are
/// flattened to within one quarter canvas pixel or return a closed error before
/// a partial path can escape.
pub fn canvas_path(path: &GlyphPath, placement: GlyphPlacement) -> Result<Path, GlyphRenderError> {
    let mut result = Path::new();
    let mut vertex_count = 0;
    for contour in 0..path.contour_count() {
        let start = path
            .contour_start(contour)
            .expect("glyph paths keep a start for every contour");
        let segments = path
            .segment_slice(contour)
            .expect("glyph paths keep segments for every contour");
        let start = placement.map(start);
        let mut points = Vec::with_capacity(segments.len().saturating_add(1));
        push_vertex(&mut points, start, &mut vertex_count)?;
        let mut current = start;
        for segment in segments {
            match *segment {
                GlyphSegment::LineTo { to } => {
                    current = placement.map(to);
                    push_vertex(&mut points, current, &mut vertex_count)?;
                }
                GlyphSegment::QuadraticTo { control, to } => {
                    let curve = Quadratic::new(current, placement.map(control), placement.map(to));
                    append_quadratic(curve, &mut points, &mut vertex_count)?;
                    current = curve.to;
                }
            }
        }
        if points.last() == Some(&start) {
            points.pop();
            vertex_count -= 1;
        }
        result.push_owned_contour(points);
    }
    Ok(result)
}

pub(crate) fn append_quadratic(
    curve: Quadratic,
    points: &mut Vec<Point>,
    vertex_count: &mut usize,
) -> Result<(), GlyphRenderError> {
    let mut stack = [Quadratic::empty(); SUBDIVISION_STACK_CAPACITY];
    stack[0] = curve;
    let mut stack_len = 1;
    while stack_len > 0 {
        stack_len -= 1;
        let current = stack[stack_len];
        if current.is_flat_enough() {
            push_vertex(points, current.to, vertex_count)?;
            continue;
        }
        if current.depth == MAX_SUBDIVISION_DEPTH {
            return Err(GlyphRenderError::TooComplex);
        }
        let (first, second) = current.split();
        stack[stack_len] = second;
        stack[stack_len + 1] = first;
        stack_len += 2;
    }
    Ok(())
}

fn push_vertex(
    points: &mut Vec<Point>,
    point: Point,
    vertex_count: &mut usize,
) -> Result<(), GlyphRenderError> {
    if points.last() == Some(&point) {
        return Ok(());
    }
    if *vertex_count == MAX_GLYPH_VERTICES {
        return Err(GlyphRenderError::TooComplex);
    }
    points.push(point);
    *vertex_count += 1;
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct Quadratic {
    from: Point,
    control: Point,
    to: Point,
    depth: u8,
}

impl Quadratic {
    const fn empty() -> Self {
        Self {
            from: Point::new(0.0, 0.0),
            control: Point::new(0.0, 0.0),
            to: Point::new(0.0, 0.0),
            depth: 0,
        }
    }

    pub(crate) const fn new(from: Point, control: Point, to: Point) -> Self {
        Self {
            from,
            control,
            to,
            depth: 0,
        }
    }

    fn is_flat_enough(self) -> bool {
        let chord = self.from.to(self.to);
        let chord_length_squared = chord.dot(chord);
        let control_offset = self.from.to(self.control);
        if chord_length_squared <= f32::EPSILON {
            return control_offset.dot(control_offset) <= MAX_DEVIATION_SQUARED;
        }
        let cross = control_offset.x * chord.y - control_offset.y * chord.x;
        cross * cross <= MAX_DEVIATION_SQUARED * chord_length_squared
    }

    fn split(self) -> (Self, Self) {
        let first_control = self.from.lerp(self.control, 0.5);
        let second_control = self.control.lerp(self.to, 0.5);
        let middle = first_control.lerp(second_control, 0.5);
        (
            Self {
                from: self.from,
                control: first_control,
                to: middle,
                depth: self.depth + 1,
            },
            Self {
                from: middle,
                control: second_control,
                to: self.to,
                depth: self.depth + 1,
            },
        )
    }
}
