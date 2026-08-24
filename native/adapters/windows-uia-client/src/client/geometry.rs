//! Screen geometry returned by a fixed Windows accessibility diagnostic.

use crate::raw;

/// A UI Automation screen rectangle in physical pixels.
///
/// It is copied from Windows only inside the host diagnostic adapter. The
/// platform protocol and application SDK expose neither this geometry nor an
/// operation that can choose a coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAutomationRect {
    /// Left edge in screen pixels.
    pub left: i32,
    /// Top edge in screen pixels.
    pub top: i32,
    /// Right edge in screen pixels.
    pub right: i32,
    /// Bottom edge in screen pixels.
    pub bottom: i32,
}

impl UiAutomationRect {
    /// Returns whether Windows reported no visible area for this rectangle.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// Returns whether another non-empty rectangle fits inside this one.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    pub(super) fn center(self) -> Option<raw::Point> {
        if self.is_empty() {
            return None;
        }
        Some(raw::Point {
            x: ((i64::from(self.left) + i64::from(self.right)) / 2) as i32,
            y: ((i64::from(self.top) + i64::from(self.bottom)) / 2) as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::UiAutomationRect;

    #[test]
    fn rectangle_containment_rejects_empty_and_outside_rectangles() {
        let root = UiAutomationRect {
            left: -100,
            top: -50,
            right: 300,
            bottom: 250,
        };
        assert!(root.contains(UiAutomationRect {
            left: -100,
            top: -50,
            right: 300,
            bottom: 250,
        }));
        assert!(!root.contains(UiAutomationRect {
            left: 300,
            top: 0,
            right: 300,
            bottom: 20,
        }));
        assert!(!root.contains(UiAutomationRect {
            left: -101,
            top: 0,
            right: 20,
            bottom: 20,
        }));
    }

    #[test]
    fn rectangle_centre_uses_wide_intermediate_arithmetic() {
        let rectangle = UiAutomationRect {
            left: i32::MIN,
            top: i32::MIN,
            right: i32::MAX,
            bottom: i32::MAX,
        };
        let centre = rectangle.center().expect("spanning rectangle has a centre");
        assert_eq!(centre.x, 0);
        assert_eq!(centre.y, 0);
    }
}
