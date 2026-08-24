//! Retained local scrolling and scrollbar geometry for the UI Lab.

use super::*;

impl UiLab {
    /// Moves the first visible diagnostic scroll viewport by one page.
    ///
    /// This is local Windows Lab behavior only. It does not produce an
    /// application event or carry native authority.
    pub(in crate::win32) fn scroll_page(&mut self, width: f32, height: f32, forward: bool) -> bool {
        let layout = self.layout(width, height);
        let Some(metrics) = Self::first_overflowing_scroll_metric(&layout) else {
            return false;
        };
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_page(forward, metrics.viewport_height(), metrics.content_height());
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Moves the first visible diagnostic scroll viewport by one local line.
    pub(in crate::win32) fn scroll_line(&mut self, width: f32, height: f32, forward: bool) -> bool {
        let layout = self.layout(width, height);
        let Some(metrics) = Self::first_overflowing_scroll_metric(&layout) else {
            return false;
        };
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_line(forward, metrics.viewport_height(), metrics.content_height());
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Converts one native wheel delta into owned whole-line movement.
    pub(in crate::win32) fn scroll_wheel_delta(
        &mut self,
        width: f32,
        height: f32,
        delta: i32,
    ) -> bool {
        let lines = self.wheel.push(delta);
        let forward = lines < 0;
        (0..lines.unsigned_abs()).any(|_| self.scroll_line(width, height, forward))
    }

    /// Begins one host-local scrollbar thumb drag when the pointer is on it.
    ///
    /// The returned state contains no raw pointer data after the caller's
    /// current message. The Win32 owner uses it only to decide whether to
    /// capture the pointer for this native window.
    pub(in crate::win32) fn begin_scrollbar_drag(
        &mut self,
        width: f32,
        height: f32,
        at: Point,
    ) -> bool {
        let surface = Surface::new(width, height);
        let Some((scrollbar, _)) = self.first_scrollbar(width, height) else {
            return false;
        };
        let Some(ScrollbarHit::Thumb { grab_offset_y }) =
            scrollbar.hit_test(surface.to_ui_point(at))
        else {
            return false;
        };
        self.scrollbar_drag = Some(ScrollbarDrag {
            id: scrollbar.id().clone(),
            grab_offset_y,
        });
        self.scrollbar_release_pending = true;
        self.hovered = None;
        true
    }

    /// Applies one captured pointer position to the retained scrollbar offset.
    pub(in crate::win32) fn drag_scrollbar(&mut self, width: f32, height: f32, at: Point) -> bool {
        let Some(drag) = self.scrollbar_drag.clone() else {
            return false;
        };
        let Some((scrollbar, metrics)) = self.first_scrollbar(width, height) else {
            return false;
        };
        if scrollbar.id() != &drag.id {
            return false;
        }
        let surface = Surface::new(width, height);
        let requested =
            scrollbar.offset_for_thumb_grab(surface.to_ui_point(at).y, drag.grab_offset_y);
        let changed = self
            .scroll_offsets
            .entry(metrics.id().clone())
            .or_default()
            .scroll_to(
                requested,
                metrics.viewport_height(),
                metrics.content_height(),
            );
        if changed {
            self.hovered = None;
        }
        changed
    }

    /// Stops a host-local thumb drag after a release or capture loss.
    pub(in crate::win32) fn end_scrollbar_drag(&mut self) -> bool {
        let ended = self.scrollbar_drag.take().is_some() || self.scrollbar_release_pending;
        self.scrollbar_release_pending = false;
        ended
    }

    /// Moves one host-owned scrollbar by a page when its track was pressed.
    ///
    /// A thumb hit is also consumed, so an opaque overlay cannot activate an
    /// action that happens to be painted beneath it.
    pub(in crate::win32) fn page_scrollbar_at(
        &mut self,
        width: f32,
        height: f32,
        at: Point,
    ) -> bool {
        let surface = Surface::new(width, height);
        let Some((scrollbar, metrics)) = self.first_scrollbar(width, height) else {
            return false;
        };
        let Some(hit) = scrollbar.hit_test(surface.to_ui_point(at)) else {
            return false;
        };
        let changed = match hit {
            ScrollbarHit::Thumb { .. } => false,
            ScrollbarHit::TrackBefore => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_page(false, metrics.viewport_height(), metrics.content_height()),
            ScrollbarHit::TrackAfter => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_page(true, metrics.viewport_height(), metrics.content_height()),
        };
        if changed {
            self.hovered = None;
        }
        true
    }

    /// Clamps retained scroll positions after a native size change.
    pub(in crate::win32) fn clamp_scroll_offsets(&mut self, width: f32, height: f32) {
        let metrics = self.layout(width, height).scroll_metrics().to_vec();
        for metric in metrics {
            self.scroll_offsets
                .entry(metric.id().clone())
                .or_default()
                .clamp(metric.viewport_height(), metric.content_height());
        }
    }

    pub(in crate::win32) fn layout(&self, width: f32, height: f32) -> UiLayout {
        let surface = Surface::new(width, height);
        self.document.layout_with_scroll_offsets(
            surface.bounds(),
            &WindowsTextMeasurer,
            &self.scroll_offsets,
        )
    }

    pub(in crate::win32) fn first_scrollbar(
        &self,
        width: f32,
        height: f32,
    ) -> Option<(Scrollbar, anodrel_ui::UiScrollMetrics)> {
        let layout = self.layout(width, height);
        self.first_scrollbar_in_layout(&layout)
    }

    pub(in crate::win32) fn first_scrollbar_in_layout(
        &self,
        layout: &UiLayout,
    ) -> Option<(Scrollbar, anodrel_ui::UiScrollMetrics)> {
        let metrics = Self::first_overflowing_scroll_metric(layout)?;
        let viewport = layout.bounds(metrics.id())?;
        let offset = self
            .scroll_offsets
            .get(metrics.id())
            .copied()
            .map_or(0.0, |state| state.offset_y());
        Scrollbar::from_metric(&metrics, viewport, offset).map(|scrollbar| (scrollbar, metrics))
    }

    pub(in crate::win32) fn first_overflowing_scroll_metric(
        layout: &UiLayout,
    ) -> Option<anodrel_ui::UiScrollMetrics> {
        layout
            .scroll_metrics()
            .iter()
            .find(|metrics| {
                layout.bounds(metrics.id()).is_some()
                    && UiScrollState::maximum_offset(
                        metrics.viewport_height(),
                        metrics.content_height(),
                    ) > 0.0
            })
            .cloned()
    }
}
