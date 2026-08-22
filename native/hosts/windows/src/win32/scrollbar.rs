//! Pure geometry and pointer interpretation for one host-owned scrollbar.
//!
//! The scrollbar is a Windows presentation detail over the existing portable
//! scroll metrics. It knows no document tree, canvas, native handle, input
//! message, or application protocol route.

use anodrel_ui::{ElementId, UiPoint, UiRect, UiScrollMetrics, UiScrollState};

/// Logical width of the direct-rendered scrollbar overlay.
const WIDTH: f32 = 10.0;
/// Inset between a scroll viewport's edge and its scrollbar track.
const EDGE_INSET: f32 = 4.0;
/// Smallest visible thumb length in logical pixels.
const MIN_THUMB_LENGTH: f32 = 24.0;

/// One finite vertical scrollbar derived from a current viewport metric.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Scrollbar {
    id: ElementId,
    track: UiRect,
    thumb: UiRect,
    maximum_offset: f32,
}

impl Scrollbar {
    /// Builds one overlay scrollbar when the supplied viewport overflows.
    pub(super) fn from_metric(
        metric: &UiScrollMetrics,
        viewport: UiRect,
        current_offset: f32,
    ) -> Option<Self> {
        let viewport_height = metric.viewport_height();
        let content_height = metric.content_height();
        let maximum_offset = UiScrollState::maximum_offset(viewport_height, content_height);
        if maximum_offset <= 0.0 || viewport.is_empty() {
            return None;
        }

        let track = UiRect::new(
            viewport.right - EDGE_INSET - WIDTH,
            viewport.top + EDGE_INSET,
            viewport.right - EDGE_INSET,
            viewport.bottom - EDGE_INSET,
        );
        if track.is_empty() {
            return None;
        }

        let thumb_height = (track.height() * viewport_height / content_height)
            .clamp(MIN_THUMB_LENGTH, track.height());
        let travel = track.height() - thumb_height;
        let offset = if current_offset.is_finite() {
            current_offset.clamp(0.0, maximum_offset)
        } else {
            0.0
        };
        let thumb_top = track.top + travel * offset / maximum_offset;
        let thumb = UiRect::from_size(track.left, thumb_top, track.width(), thumb_height);

        Some(Self {
            id: metric.id().clone(),
            track,
            thumb,
            maximum_offset,
        })
    }

    /// Returns the viewport identity whose local offset this control changes.
    pub(super) fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the finite visible track.
    pub(super) const fn track(&self) -> UiRect {
        self.track
    }

    /// Returns the finite visible thumb.
    pub(super) const fn thumb(&self) -> UiRect {
        self.thumb
    }

    /// Classifies one point without returning any input data to an application.
    pub(super) fn hit_test(&self, point: UiPoint) -> Option<ScrollbarHit> {
        if self.thumb.contains(point) {
            Some(ScrollbarHit::Thumb {
                grab_offset_y: point.y - self.thumb.top,
            })
        } else if self.track.contains(point) {
            Some(if point.y < self.thumb.top {
                ScrollbarHit::TrackBefore
            } else {
                ScrollbarHit::TrackAfter
            })
        } else {
            None
        }
    }

    /// Maps one host-local thumb position to the existing bounded scroll range.
    pub(super) fn offset_for_thumb_grab(&self, pointer_y: f32, grab_offset_y: f32) -> f32 {
        if !pointer_y.is_finite() || !grab_offset_y.is_finite() {
            return 0.0;
        }
        let travel = self.track.height() - self.thumb.height();
        if travel <= 0.0 || !travel.is_finite() {
            return 0.0;
        }
        let thumb_top = (pointer_y - grab_offset_y)
            .clamp(self.track.top, self.track.bottom - self.thumb.height());
        self.maximum_offset * (thumb_top - self.track.top) / travel
    }
}

/// One local result from pointing at a direct-rendered scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ScrollbarHit {
    /// Pointer pressed on the thumb and reports its in-thumb vertical offset.
    Thumb {
        /// Vertical distance from the thumb's top edge.
        grab_offset_y: f32,
    },
    /// Pointer pressed on the track before the thumb.
    TrackBefore,
    /// Pointer pressed on the track after the thumb.
    TrackAfter,
}

#[cfg(test)]
mod tests {
    use super::{Scrollbar, ScrollbarHit};
    use anodrel_ui::{
        ElementId, Scroll, Text, TextMeasurer, UiDocument, UiNode, UiPoint, UiRect,
        UiScrollMetrics, UiSize,
    };

    fn metric(viewport_height: f32, content_height: f32) -> UiScrollMetrics {
        let id = ElementId::new("viewport").expect("fixed ID is valid");
        let document = UiDocument::new(UiNode::Scroll(Scroll::new(
            id,
            UiNode::Text(
                Text::new(
                    ElementId::new("content").expect("fixed ID is valid"),
                    "content",
                    12,
                )
                .expect("fixed content is valid"),
            ),
        )))
        .expect("fixed document is valid");
        document
            .layout(
                UiRect::from_size(20.0, 30.0, 300.0, viewport_height),
                &FixedMeasurer { content_height },
            )
            .scroll_metrics()
            .first()
            .expect("root scroll reports metrics")
            .clone()
    }

    fn viewport() -> UiRect {
        UiRect::from_size(20.0, 30.0, 300.0, 180.0)
    }

    struct FixedMeasurer {
        content_height: f32,
    }

    impl TextMeasurer for FixedMeasurer {
        fn measure(&self, _value: &str, _font_size: u16) -> UiSize {
            UiSize::new(100.0, self.content_height)
        }
    }

    #[test]
    fn omits_a_control_when_content_does_not_overflow() {
        assert_eq!(
            Scrollbar::from_metric(&metric(180.0, 180.0), viewport(), 0.0),
            None
        );
        assert_eq!(
            Scrollbar::from_metric(&metric(180.0, 90.0), viewport(), 0.0),
            None
        );
    }

    #[test]
    fn maps_bounded_offsets_to_a_finite_thumb_inside_its_track() {
        let top = Scrollbar::from_metric(&metric(180.0, 720.0), viewport(), 0.0)
            .expect("overflow produces a control");
        let bottom = Scrollbar::from_metric(&metric(180.0, 720.0), viewport(), 9999.0)
            .expect("overflow produces a control");

        assert!(top.track().contains(UiPoint::new(
            (top.track().left + top.track().right) / 2.0,
            (top.track().top + top.track().bottom) / 2.0,
        )));
        assert_eq!(top.thumb().top, top.track().top);
        assert_eq!(bottom.thumb().bottom, bottom.track().bottom);
        assert!(top.thumb().height() >= 24.0);
        assert_eq!(top.id().as_str(), "viewport");
    }

    #[test]
    fn classifies_track_sides_and_maps_a_drag_to_the_full_offset_range() {
        let viewport = UiRect::from_size(20.0, 30.0, 300.0, 100.0);
        let scrollbar = Scrollbar::from_metric(&metric(100.0, 500.0), viewport, 0.0)
            .expect("overflow produces a control");
        let centre_x = (scrollbar.track().left + scrollbar.track().right) / 2.0;

        assert_eq!(
            scrollbar.hit_test(UiPoint::new(centre_x, scrollbar.track().top)),
            Some(ScrollbarHit::Thumb { grab_offset_y: 0.0 })
        );
        assert_eq!(
            scrollbar.hit_test(UiPoint::new(centre_x, scrollbar.track().bottom - 1.0)),
            Some(ScrollbarHit::TrackAfter)
        );
        assert_eq!(scrollbar.offset_for_thumb_grab(-100.0, 0.0), 0.0);
        assert_eq!(
            scrollbar.offset_for_thumb_grab(scrollbar.track().bottom + 100.0, 0.0),
            400.0
        );
    }
}
