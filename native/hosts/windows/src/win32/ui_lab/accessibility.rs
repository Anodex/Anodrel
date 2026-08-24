//! UI Automation snapshots and host-owned accessibility routes for the UI Lab.
//!
//! The provider receives revision-bound mailbox routes; this module revalidates
//! each request against the exact current retained layout before it changes local state.

use std::collections::BTreeSet;

use super::*;

/// Collects the semantic children whose nearest scroll ancestor is `selected`.
///
/// The walk stays inside the host because a UI Automation provider must never
/// infer scroll ownership from control types, geometry, or application data.
/// A nested viewport is itself a child of its outer viewport, but its contents
/// belong to the nested context and remain outside this first route.
fn collect_scroll_item_ids(
    node: &UiNode,
    nearest_scroll: Option<ElementId>,
    selected: &ElementId,
    laid_out: &BTreeSet<ElementId>,
    output: &mut Vec<ElementId>,
) {
    if nearest_scroll.as_ref() == Some(selected) && laid_out.contains(node.id()) {
        output.push(node.id().clone());
    }
    match node {
        UiNode::Stack(stack) => {
            for child in stack.children() {
                collect_scroll_item_ids(child, nearest_scroll.clone(), selected, laid_out, output);
            }
        }
        UiNode::Scroll(scroll) => collect_scroll_item_ids(
            scroll.child(),
            Some(scroll.id().clone()),
            selected,
            laid_out,
            output,
        ),
        UiNode::Text(_) | UiNode::Status(_) | UiNode::Action(_) | UiNode::Field(_) => {}
    }
}

/// Calculates the existing retained offset needed to reveal an item.
///
/// Coordinates are from the current layout, so the item's paint rectangle has
/// already moved by the current retained offset. This calculation only chooses
/// a candidate; [`UiScrollState::scroll_to`] remains the finite clamp.
pub(in crate::win32) fn scroll_into_view_offset(
    viewport: UiRect,
    item: UiRect,
    current_offset: f32,
) -> Option<f32> {
    if viewport.is_empty() || item.is_empty() || !current_offset.is_finite() {
        return None;
    }
    let displacement = if item.height() >= viewport.height() || item.top < viewport.top {
        item.top - viewport.top
    } else if item.bottom > viewport.bottom {
        item.bottom - viewport.bottom
    } else {
        0.0
    };
    let requested = current_offset + displacement;
    requested.is_finite().then_some(requested)
}

impl UiLab {
    /// Derives the accessibility semantics for this view's current layout.
    ///
    /// The same layout the surface draws produces these, so what a screen
    /// reader is told cannot drift from what is on screen.
    pub(in crate::win32) fn accessibility_snapshot(
        &self,
        width: f32,
        height: f32,
    ) -> anodrel_ui::UiAccessibilitySnapshot {
        self.document
            .accessibility_snapshot(&self.layout(width, height))
    }

    /// Copies the current host-owned field text for an accessibility snapshot.
    ///
    /// This contains only one value per existing field ID. The UI Automation
    /// adapter still admits it only for a matching visible Edit element, then
    /// exposes it as read-only to Windows. It never enters an application
    /// protocol route. See Decision 0071.
    pub(in crate::win32) fn accessibility_field_values(&self) -> Vec<(ElementId, String)> {
        self.fields
            .iter()
            .map(|(id, state)| (id.clone(), state.text().to_owned()))
            .collect()
    }

    /// Returns the host-owned keyboard focus to publish with a matching
    /// accessibility snapshot.
    ///
    /// The caller owns the layout-specific validation: a provider filters this
    /// ID against the same visible mapped tree it publishes, so a stale or
    /// clipped target becomes no reported focus rather than a guess.
    pub(in crate::win32) fn accessibility_focus(&self) -> Option<ElementId> {
        self.focus.focused().cloned()
    }

    /// Binds one immutable provider snapshot to this view's focus route.
    pub(in crate::win32) fn accessibility_focus_route(
        &self,
        revision: Option<anodrel_ui_session::UiDocumentRevision>,
    ) -> UiAutomationFocusRoute {
        self.automation_focus.route(revision)
    }

    /// Copies the one host-selected vertical scroll snapshot for UI Automation.
    ///
    /// It is derived from the same layout and retained offset currently drawn.
    /// A non-overflowing document has no automation scroll target.
    pub(in crate::win32) fn accessibility_scroll_snapshot(
        &self,
        width: f32,
        height: f32,
    ) -> Option<UiAutomationScrollSnapshot> {
        let layout = self.layout(width, height);
        let metrics = Self::first_overflowing_scroll_metric(&layout)?;
        let offset = self
            .scroll_offsets
            .get(metrics.id())
            .copied()
            .map_or(0.0, UiScrollState::offset_y);
        UiAutomationScrollSnapshot::new(
            metrics.id().clone(),
            metrics.viewport_height(),
            metrics.content_height(),
            offset,
        )
    }

    /// Returns the immutable semantic descendants eligible for ScrollItem.
    ///
    /// This uses the same current layout and first-visible-overflow selection
    /// as the published ScrollPattern. Fully clipped items remain eligible so
    /// accessibility navigation can request their reveal; local focus and
    /// input continue to reject their empty clipped rectangles.
    pub(in crate::win32) fn accessibility_scroll_items(
        &self,
        width: f32,
        height: f32,
    ) -> Vec<ElementId> {
        let layout = self.layout(width, height);
        let Some(metrics) = Self::first_overflowing_scroll_metric(&layout) else {
            return Vec::new();
        };
        self.scroll_item_ids_in_layout(&layout, metrics.id())
    }

    /// Binds one immutable provider snapshot to this view's scroll route.
    pub(in crate::win32) fn accessibility_scroll_route(
        &self,
        revision: Option<anodrel_ui_session::UiDocumentRevision>,
    ) -> UiAutomationScrollRoute {
        self.automation_scroll.route(revision)
    }

    /// Takes and revalidates at most one pending UI Automation focus request.
    ///
    /// `expected_revision` is `None` for the fixed diagnostic UI Lab and the
    /// current accepted document revision for an authenticated session. A
    /// successful request can leave an already-focused target in place, which
    /// is still a truthful success for `SetFocus`.
    pub(in crate::win32) fn service_accessibility_focus(
        &mut self,
        expected_revision: Option<anodrel_ui_session::UiDocumentRevision>,
        width: f32,
        height: f32,
    ) -> Option<AccessibilityFocusResult> {
        let mailbox = self.automation_focus.clone();
        let request = mailbox.take()?;
        let mut changed = false;
        let accepted = mailbox.complete_with(request.id(), || {
            if request.revision() != expected_revision {
                return false;
            }
            let Some(focus_changed) =
                self.focus_accessibility_target(width, height, request.target())
            else {
                return false;
            };
            changed = focus_changed;
            true
        })?;
        Some(AccessibilityFocusResult { accepted, changed })
    }

    pub(in crate::win32) fn focus_accessibility_target(
        &mut self,
        width: f32,
        height: f32,
        target: &ElementId,
    ) -> Option<bool> {
        let layout = self.layout(width, height);
        if !self.focus.can_focus(&layout, target) {
            return None;
        }
        Some(self.focus.focus_on(&layout, target))
    }

    /// Takes and revalidates at most one UI Automation scroll request.
    ///
    /// The provider's revision and selected viewport must still match the
    /// current view. The one accepted command changes only the established
    /// host-retained position, never application state or input.
    pub(in crate::win32) fn service_accessibility_scroll(
        &mut self,
        expected_revision: Option<anodrel_ui_session::UiDocumentRevision>,
        width: f32,
        height: f32,
    ) -> Option<AccessibilityScrollResult> {
        let mailbox = self.automation_scroll.clone();
        let request = mailbox.take()?;
        let mut changed = false;
        let accepted = mailbox.complete_with(request.id(), || {
            if request.revision() != expected_revision {
                return false;
            }
            let Some(scroll_changed) = self.scroll_accessibility_target(
                width,
                height,
                request.target(),
                request.command(),
            ) else {
                return false;
            };
            changed = scroll_changed;
            true
        })?;
        Some(AccessibilityScrollResult { accepted, changed })
    }

    pub(in crate::win32) fn scroll_accessibility_target(
        &mut self,
        width: f32,
        height: f32,
        target: &ElementId,
        command: UiAutomationScrollCommand,
    ) -> Option<bool> {
        let layout = self.layout(width, height);
        let metrics = Self::first_overflowing_scroll_metric(&layout)?;
        if metrics.id() != target {
            return None;
        }
        let changed = match command {
            UiAutomationScrollCommand::Line { forward } => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_line(forward, metrics.viewport_height(), metrics.content_height()),
            UiAutomationScrollCommand::Page { forward } => self
                .scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_page(forward, metrics.viewport_height(), metrics.content_height()),
            UiAutomationScrollCommand::Percent { percent } => {
                let maximum = UiScrollState::maximum_offset(
                    metrics.viewport_height(),
                    metrics.content_height(),
                );
                self.scroll_offsets
                    .entry(metrics.id().clone())
                    .or_default()
                    .scroll_to(
                        maximum * (percent / 100.0) as f32,
                        metrics.viewport_height(),
                        metrics.content_height(),
                    )
            }
            UiAutomationScrollCommand::ScrollIntoView { item } => {
                self.scroll_item_into_view(&layout, &metrics, target, &item)?
            }
        };
        if changed {
            self.hovered = None;
        }
        Some(changed)
    }

    fn scroll_item_into_view(
        &mut self,
        layout: &UiLayout,
        metrics: &anodrel_ui::UiScrollMetrics,
        viewport: &ElementId,
        item: &ElementId,
    ) -> Option<bool> {
        if !self
            .scroll_item_ids_in_layout(layout, viewport)
            .contains(item)
        {
            return None;
        }
        let viewport_bounds = layout.bounds(viewport)?;
        let item_bounds = layout
            .items()
            .iter()
            .find(|candidate| candidate.id() == item)?
            .paint_bounds();
        let current_offset = self
            .scroll_offsets
            .get(metrics.id())
            .copied()
            .map_or(0.0, UiScrollState::offset_y);
        let requested = scroll_into_view_offset(viewport_bounds, item_bounds, current_offset)?;
        Some(
            self.scroll_offsets
                .entry(metrics.id().clone())
                .or_default()
                .scroll_to(
                    requested,
                    metrics.viewport_height(),
                    metrics.content_height(),
                ),
        )
    }

    fn scroll_item_ids_in_layout(&self, layout: &UiLayout, viewport: &ElementId) -> Vec<ElementId> {
        let laid_out = layout
            .items()
            .iter()
            .map(|item| item.id().clone())
            .collect::<BTreeSet<_>>();
        let mut items = Vec::new();
        collect_scroll_item_ids(self.document.root(), None, viewport, &laid_out, &mut items);
        items
    }
}
