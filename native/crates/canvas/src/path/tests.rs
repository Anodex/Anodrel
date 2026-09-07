use super::{Path, signed_area};
use crate::geometry::{Point, Rect, point};

fn square() -> Path {
    Path::polygon([
        point(0.0, 0.0),
        point(10.0, 0.0),
        point(10.0, 10.0),
        point(0.0, 10.0),
    ])
}

#[test]
fn contours_with_fewer_than_three_points_are_rejected() {
    let mut path = Path::new();
    path.push_contour([point(0.0, 0.0), point(1.0, 1.0)]);
    assert!(path.is_empty());
    assert_eq!(path.bounds(), Rect::default());
}

#[test]
fn owned_contours_follow_the_same_closure_rule() {
    let mut path = Path::new();
    path.push_owned_contour(vec![point(0.0, 0.0), point(4.0, 0.0), point(0.0, 4.0)]);
    assert_eq!(
        path.contours(),
        &[vec![point(0.0, 0.0), point(4.0, 0.0), point(0.0, 4.0)]]
    );
}

#[test]
fn bounds_cover_every_contour() {
    let mut path = square();
    path.push_contour([point(20.0, 20.0), point(30.0, 20.0), point(30.0, 30.0)]);
    assert_eq!(path.bounds(), Rect::new(0.0, 0.0, 30.0, 30.0));
}

#[test]
fn inset_moves_a_square_inward_on_every_side() {
    assert_eq!(square().inset(2.0).bounds(), Rect::new(2.0, 2.0, 8.0, 8.0));
}

#[test]
fn inset_is_independent_of_winding_direction() {
    let mut reversed: Vec<Point> = square().contours()[0].clone();
    reversed.reverse();
    assert_eq!(
        Path::polygon(reversed).inset(2.0).bounds(),
        Rect::new(2.0, 2.0, 8.0, 8.0)
    );
}

#[test]
fn bevel_bands_cover_every_edge_with_outward_normals() {
    let bands = square().bevel_bands(2.0);
    assert_eq!(bands.len(), 4);
    assert!((bands[0].outward_normal.x).abs() < 1e-5);
    assert!(bands[0].outward_normal.y < 0.0);
}

#[test]
fn ring_encloses_a_reversed_inner_contour() {
    let ring = Path::ring(point(50.0, 50.0), 20.0, 5.0);
    assert_eq!(ring.contours().len(), 2);
    assert!(signed_area(&ring.contours()[0]) * signed_area(&ring.contours()[1]) < 0.0);
}

#[test]
fn rounded_rect_radius_is_clamped_to_the_shorter_side() {
    let rect = Rect::new(0.0, 0.0, 40.0, 10.0);
    let bounds = Path::rounded_rect(rect, 100.0).bounds();
    assert!((bounds.left - rect.left).abs() < 0.51);
    assert!((bounds.bottom - rect.bottom).abs() < 0.51);
}

#[test]
fn fit_unit_square_places_normalised_geometry() {
    let unit = Path::polygon([point(0.0, 0.0), point(1.0, 0.0), point(0.5, 1.0)]);
    assert_eq!(
        unit.fit_unit_square(Rect::new(10.0, 20.0, 110.0, 220.0))
            .bounds(),
        Rect::new(10.0, 20.0, 110.0, 220.0)
    );
}
