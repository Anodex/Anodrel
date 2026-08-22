//! Conversion from clipped logical layout bounds to UI Automation rectangles.

use anodrel_ui::UiRect;

/// The screen position and density of one host window's client area.
///
/// The host supplies both rather than the mapping querying them, which keeps
/// the conversion a pure function and lets it be tested at any display density
/// without creating a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientOrigin {
    left: i32,
    top: i32,
    scale: f32,
}

impl ClientOrigin {
    /// Records where a client area sits on screen and how dense it is.
    ///
    /// `scale` is the window's current factor from logical to physical pixels;
    /// a non-finite or non-positive value falls back to 1.0 rather than
    /// producing rectangles Windows would reject.
    #[must_use]
    pub fn new(left: i32, top: i32, scale: f32) -> Self {
        Self {
            left,
            top,
            scale: if scale.is_finite() && scale > 0.0 {
                scale
            } else {
                1.0
            },
        }
    }
}

/// One UI Automation bounding rectangle: left, top, width, height in physical
/// screen pixels.
///
/// UI Automation expresses these as doubles, so the conversion widens rather
/// than rounding and losing sub-pixel placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    /// Left edge in physical screen pixels.
    pub left: f64,
    /// Top edge in physical screen pixels.
    pub top: f64,
    /// Width in physical pixels; zero when the node has no visible area.
    pub width: f64,
    /// Height in physical pixels; zero when the node has no visible area.
    pub height: f64,
}

impl ScreenRect {
    /// The rectangle reported for a node with no visible area.
    pub const EMPTY: Self = Self {
        left: 0.0,
        top: 0.0,
        width: 0.0,
        height: 0.0,
    };

    /// Returns the four values in the order UI Automation expects them.
    #[must_use]
    pub const fn as_array(self) -> [f64; 4] {
        [self.left, self.top, self.width, self.height]
    }

    /// Returns whether this rectangle names no currently visible screen area.
    #[must_use]
    pub fn is_empty(self) -> bool {
        !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.0
            || self.height <= 0.0
    }
}

/// Converts one node's clipped logical bounds into a screen rectangle.
///
/// An empty rectangle stays empty instead of collapsing to a point at the
/// client origin, so a node clipped entirely out of view can never be reported
/// as something to click at the top-left corner of the window.
#[must_use]
pub fn screen_rect(bounds: UiRect, origin: ClientOrigin) -> ScreenRect {
    // `is_empty` also rejects non-finite edges, so a rectangle that reached
    // here from a degenerate layout cannot become a NaN Windows would choke on.
    if bounds.is_empty() {
        return ScreenRect::EMPTY;
    }

    let scale = f64::from(origin.scale);
    ScreenRect {
        left: f64::from(origin.left) + f64::from(bounds.left) * scale,
        top: f64::from(origin.top) + f64::from(bounds.top) * scale,
        width: f64::from(bounds.width()) * scale,
        height: f64::from(bounds.height()) * scale,
    }
}

#[cfg(test)]
mod tests {
    use anodrel_ui::UiRect;

    use super::{ClientOrigin, ScreenRect, screen_rect};

    #[test]
    fn logical_bounds_become_screen_pixels_at_the_window_position() {
        let rect = screen_rect(
            UiRect::new(10.0, 20.0, 110.0, 60.0),
            ClientOrigin::new(300, 150, 1.0),
        );
        assert_eq!(rect.as_array(), [310.0, 170.0, 100.0, 40.0]);
    }

    #[test]
    fn a_denser_display_scales_position_and_size_together() {
        // Only the offsets inside the client area scale; the client origin is
        // already in physical screen pixels.
        let rect = screen_rect(
            UiRect::new(10.0, 20.0, 110.0, 60.0),
            ClientOrigin::new(300, 150, 2.0),
        );
        assert_eq!(rect.as_array(), [320.0, 190.0, 200.0, 80.0]);
    }

    #[test]
    fn a_node_clipped_out_of_view_reports_no_rectangle() {
        // Collapsing to a point would put a clickable target at the window's
        // top-left corner for something that is not on screen at all.
        for empty in [
            UiRect::new(10.0, 20.0, 10.0, 60.0),
            UiRect::new(10.0, 20.0, 110.0, 20.0),
        ] {
            assert_eq!(
                screen_rect(empty, ClientOrigin::new(300, 150, 1.5)),
                ScreenRect::EMPTY
            );
        }
    }

    #[test]
    fn an_unusable_scale_falls_back_rather_than_producing_a_bad_rectangle() {
        for bad in [0.0, -2.0, f32::NAN, f32::INFINITY] {
            let rect = screen_rect(
                UiRect::new(0.0, 0.0, 10.0, 10.0),
                ClientOrigin::new(0, 0, bad),
            );
            assert_eq!(rect.as_array(), [0.0, 0.0, 10.0, 10.0], "scale {bad}");
        }
    }

    #[test]
    fn a_non_finite_rectangle_never_reaches_windows() {
        assert_eq!(
            screen_rect(
                UiRect::new(f32::NAN, 0.0, 10.0, 10.0),
                ClientOrigin::new(0, 0, 1.0)
            ),
            ScreenRect::EMPTY
        );
    }

    #[test]
    fn a_window_at_a_negative_screen_position_still_converts() {
        // A window on a monitor left of the primary one has negative screen
        // coordinates, which are ordinary rather than an error.
        let rect = screen_rect(
            UiRect::new(0.0, 0.0, 50.0, 25.0),
            ClientOrigin::new(-1920, -200, 1.0),
        );
        assert_eq!(rect.as_array(), [-1920.0, -200.0, 50.0, 25.0]);
    }
}
