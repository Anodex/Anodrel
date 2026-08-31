//! Deterministic conversion from TrueType contour points to quadratic segments.

use crate::outline::{GlyphOutline, GlyphPath, GlyphPathPoint, GlyphPoint, GlyphSegment};

impl GlyphOutline {
    /// Converts this validated simple outline into exact closed quadratic paths.
    pub fn quadratic_path(&self) -> GlyphPath {
        let mut contour_starts = Vec::with_capacity(self.contour_count());
        let mut contour_segment_ends = Vec::with_capacity(self.contour_count());
        let mut segments = Vec::with_capacity(self.point_count() + self.contour_count());
        for contour in 0..self.contour_count() {
            let Some(points) = self.point_slice(contour) else {
                continue;
            };
            let Some(start) = contour_start(points) else {
                continue;
            };
            contour_starts.push(start);
            append_contour_segments(points, start, &mut segments);
            contour_segment_ends.push(segments.len());
        }
        GlyphPath::new(contour_starts, contour_segment_ends, segments)
    }
}

fn contour_start(points: &[GlyphPoint]) -> Option<GlyphPathPoint> {
    let first = points.first()?;
    if first.is_on_curve() {
        return Some(exact_point(*first));
    }
    let last = *points.last()?;
    if last.is_on_curve() {
        return Some(exact_point(last));
    }
    Some(midpoint(exact_point(last), exact_point(*first)))
}

fn append_contour_segments(
    points: &[GlyphPoint],
    start: GlyphPathPoint,
    segments: &mut Vec<GlyphSegment>,
) {
    let skip_first = points[0].is_on_curve();
    let mut current = start;
    let mut control = None;
    for source_point in points.iter().skip(usize::from(skip_first)) {
        let point = exact_point(*source_point);
        if source_point.is_on_curve() {
            if let Some(control) = control.take() {
                segments.push(GlyphSegment::QuadraticTo { control, to: point });
            } else {
                segments.push(GlyphSegment::LineTo { to: point });
            }
            current = point;
        } else if let Some(previous_control) = control.replace(point) {
            let implied = midpoint(previous_control, point);
            segments.push(GlyphSegment::QuadraticTo {
                control: previous_control,
                to: implied,
            });
            current = implied;
        }
    }
    if let Some(control) = control {
        segments.push(GlyphSegment::QuadraticTo { control, to: start });
    } else if current != start {
        segments.push(GlyphSegment::LineTo { to: start });
    }
}

fn exact_point(point: GlyphPoint) -> GlyphPathPoint {
    GlyphPathPoint::new(i32::from(point.x()) * 2, i32::from(point.y()) * 2)
}

fn midpoint(first: GlyphPathPoint, second: GlyphPathPoint) -> GlyphPathPoint {
    GlyphPathPoint::new(
        (first.x_twice() + second.x_twice()) / 2,
        (first.y_twice() + second.y_twice()) / 2,
    )
}
