//! Authored small-size geometry for the Anodrel mark.

use anodrel_canvas::{Color, Paint, Path, Point, Rect, Stop, point};

use crate::palette;

/// One of the four pieces the mark is cut into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// The peak of the `A`.
    Apex,
    /// The descending stroke on the left.
    LeftLeg,
    /// The descending stroke on the right.
    RightLeg,
    /// The chevron forming the crossbar.
    Crossbar,
}

impl Piece {
    /// Every piece, in painting order from back to front.
    pub const ALL: [Self; 4] = [Self::LeftLeg, Self::RightLeg, Self::Crossbar, Self::Apex];

    /// Returns the piece's outline in the unit square.
    ///
    /// The mark fills the square exactly: the apex touches the top edge and the
    /// legs touch the bottom and both sides. Callers position it by choosing
    /// bounds, never by editing these numbers.
    #[must_use]
    pub fn unit_path(self) -> Path {
        match self {
            Self::Apex => Path::polygon([
                point(0.5000, 0.0000),
                point(0.7090, 0.4180),
                point(0.5580, 0.4320),
                point(0.5000, 0.2820),
                point(0.4420, 0.4320),
                point(0.2910, 0.4180),
            ]),
            Self::LeftLeg => Path::polygon(LEFT_LEG.map(|(x, y)| point(x, y))),
            Self::RightLeg => Path::polygon(mirrored(&LEFT_LEG)),
            Self::Crossbar => Path::polygon([
                point(0.5000, 0.6520),
                point(0.6970, 1.0000),
                point(0.6040, 1.0000),
                point(0.5000, 0.8000),
                point(0.3960, 1.0000),
                point(0.3030, 1.0000),
            ]),
        }
    }
}

/// The left leg, wound clockwise from its upper outer corner.
///
/// The final pair is the chamfer that blunts the outer foot, matching the way
/// the apex and crossbar terminate.
const LEFT_LEG: [(f32, f32); 5] = [
    (0.2610, 0.4780),
    (0.4112, 0.4920),
    (0.2150, 1.0000),
    (0.0750, 1.0000),
    (0.0300, 0.9400),
];

/// Mirrors a contour about the vertical centre line, preserving its winding.
fn mirrored(points: &[(f32, f32)]) -> Vec<Point> {
    points
        .iter()
        .rev()
        .map(|(x, y)| point(1.0 - x, *y))
        .collect()
}

/// Returns every piece fitted to `bounds`.
#[must_use]
pub fn pieces(bounds: Rect) -> Vec<(Piece, Path)> {
    Piece::ALL
        .iter()
        .map(|piece| (*piece, piece.unit_path().fit_unit_square(bounds)))
        .collect()
}

/// Returns all four pieces as one multi-contour path, fitted to `bounds`.
///
/// This is the shape to glow or shadow: one mask covering the whole mark rather
/// than four overlapping ones.
#[must_use]
pub fn silhouette(bounds: Rect) -> Path {
    let mut path = Path::new();
    for piece in Piece::ALL {
        for contour in piece.unit_path().fit_unit_square(bounds).contours() {
            path.push_contour(contour.clone());
        }
    }
    path
}

/// Returns the violet-to-blue face gradient spanning `bounds` horizontally.
#[must_use]
pub fn face_paint(bounds: Rect) -> Paint {
    Paint::linear(
        point(bounds.left, 0.0),
        point(bounds.right, 0.0),
        palette::mark_ramp()
            .map(|(position, color)| Stop::new(position, color))
            .to_vec(),
    )
}

/// Returns the vertical shading laid over each piece to give it depth.
///
/// The face gradient runs horizontally, so a separate low-opacity vertical pass
/// is what keeps a tall piece from looking like flat tape.
#[must_use]
pub fn depth_paint(bounds: Rect) -> Paint {
    Paint::linear(
        point(0.0, bounds.top),
        point(0.0, bounds.bottom),
        vec![
            Stop::new(0.0, Color::WHITE.with_alpha(18)),
            Stop::new(0.45, Color::TRANSPARENT),
            Stop::new(1.0, Color::BLACK.with_alpha(48)),
        ],
    )
}
