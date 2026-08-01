//! Bounded, renderer-neutral vertical scroll state.

/// The owned default distance for one logical line scroll.
pub const DEFAULT_SCROLL_LINE: f32 = 40.0;

/// Mutable vertical position for one host-owned scroll viewport.
///
/// The state contains no element identity, document, input device, timer,
/// renderer, callback, or operating-system handle. A host supplies current
/// viewport and content extents whenever it scrolls or relayouts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiScrollState {
    offset_y: f32,
}

impl UiScrollState {
    /// Returns the current downward logical-pixel offset.
    #[must_use]
    pub const fn offset_y(self) -> f32 {
        self.offset_y
    }

    /// Returns the largest valid offset for the supplied vertical extents.
    #[must_use]
    pub fn maximum_offset(viewport_height: f32, content_height: f32) -> f32 {
        (finite_nonnegative(content_height) - finite_nonnegative(viewport_height)).max(0.0)
    }

    /// Moves by a logical-pixel delta and returns whether the position changed.
    pub fn scroll_by(&mut self, delta_y: f32, viewport_height: f32, content_height: f32) -> bool {
        self.scroll_to(
            self.offset_y + finite_delta(delta_y),
            viewport_height,
            content_height,
        )
    }

    /// Moves by one owned logical line in the selected direction.
    pub fn scroll_line(
        &mut self,
        forward: bool,
        viewport_height: f32,
        content_height: f32,
    ) -> bool {
        self.scroll_by(
            if forward {
                DEFAULT_SCROLL_LINE
            } else {
                -DEFAULT_SCROLL_LINE
            },
            viewport_height,
            content_height,
        )
    }

    /// Moves by one current viewport and returns whether the position changed.
    pub fn scroll_page(
        &mut self,
        forward: bool,
        viewport_height: f32,
        content_height: f32,
    ) -> bool {
        let page = finite_nonnegative(viewport_height);
        self.scroll_by(
            if forward { page } else { -page },
            viewport_height,
            content_height,
        )
    }

    /// Sets a requested position after clamping it to current extents.
    pub fn scroll_to(
        &mut self,
        requested_offset: f32,
        viewport_height: f32,
        content_height: f32,
    ) -> bool {
        if !requested_offset.is_finite() {
            return false;
        }
        let next = finite_nonnegative(requested_offset)
            .min(Self::maximum_offset(viewport_height, content_height));
        if self.offset_y == next {
            return false;
        }
        self.offset_y = next;
        true
    }

    /// Re-clamps the position after a viewport or content-size change.
    pub fn clamp(&mut self, viewport_height: f32, content_height: f32) -> bool {
        self.scroll_to(self.offset_y, viewport_height, content_height)
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_delta(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_SCROLL_LINE, UiScrollState};

    #[test]
    fn clamps_to_zero_when_content_fits_the_viewport() {
        let mut scroll = UiScrollState::default();
        assert!(!scroll.scroll_by(20.0, 100.0, 100.0));
        assert_eq!(scroll.offset_y(), 0.0);
        assert_eq!(UiScrollState::maximum_offset(100.0, 90.0), 0.0);
    }

    #[test]
    fn clamps_line_page_and_absolute_moves_to_the_available_range() {
        let mut scroll = UiScrollState::default();
        assert!(scroll.scroll_line(true, 100.0, 250.0));
        assert_eq!(scroll.offset_y(), DEFAULT_SCROLL_LINE);
        assert!(scroll.scroll_page(true, 100.0, 250.0));
        assert_eq!(scroll.offset_y(), 140.0);
        assert!(scroll.scroll_to(999.0, 100.0, 250.0));
        assert_eq!(scroll.offset_y(), 150.0);
        assert!(scroll.scroll_line(false, 100.0, 250.0));
        assert_eq!(scroll.offset_y(), 110.0);
    }

    #[test]
    fn relayout_clamps_a_stale_position_and_rejects_nonfinite_input() {
        let mut scroll = UiScrollState::default();
        assert!(scroll.scroll_to(100.0, 100.0, 300.0));
        assert!(scroll.clamp(250.0, 300.0));
        assert_eq!(scroll.offset_y(), 50.0);
        assert!(!scroll.scroll_by(f32::NAN, 250.0, 300.0));
        assert!(!scroll.scroll_to(f32::INFINITY, 250.0, 300.0));
        assert_eq!(scroll.offset_y(), 50.0);
    }
}
