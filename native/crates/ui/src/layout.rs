//! Deterministic UI layout and semantic action hit testing.

use std::collections::BTreeMap;

use crate::{
    Action, Axis, ElementId, Field, Scroll, Stack, UiDocument, UiNode, UiPoint, UiRect,
    UiScrollState, UiSize, wrap_text, wrapped_height,
};

mod measure;

use measure::{bounded_text_bounds, intrinsic_size};

/// Horizontal padding applied to every action label, on each side.
pub const ACTION_HORIZONTAL_PADDING: f32 = 16.0;
/// Vertical padding applied to every action label, on each side.
pub const ACTION_VERTICAL_PADDING: f32 = 12.0;
/// The smallest action height in logical pixels.
pub const ACTION_MINIMUM_HEIGHT: f32 = 36.0;

/// Horizontal padding inside a field's box, on each side.
pub const FIELD_HORIZONTAL_PADDING: f32 = 12.0;
/// Vertical padding inside a field's box, on each side.
pub const FIELD_VERTICAL_PADDING: f32 = 10.0;
/// The smallest field height in logical pixels.
///
/// Slightly taller than an action's: a field is a pointer target that also has
/// to hold a caret without the text touching its edge.
pub const FIELD_MINIMUM_HEIGHT: f32 = 40.0;

/// Host-owned measurement for the UI's plain text.
///
/// Font selection and shaping remain an operating-system concern. Non-finite
/// or negative results are treated as zero by the layout engine.
pub trait TextMeasurer {
    /// Measures one validated, single-line text value at the requested size.
    fn measure(&self, text: &str, font_size: u16) -> UiSize;
}

/// The kind of a visible laid-out element.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiLayoutKind {
    /// A stack container.
    Stack,
    /// A vertical scroll viewport.
    Scroll,
    /// A non-interactive text run.
    Text,
    /// A visible non-interactive semantic status result.
    Status,
    /// A semantic action.
    Action,
    /// A single-line field a person can type into.
    Field,
}

/// Host-owned scroll positions keyed by scroll-viewport element ID.
///
/// The layout engine reads but never mutates these positions. It independently
/// clamps each supplied value against the current measured extents, while the
/// returned [`UiScrollMetrics`] lets the host update its retained state after a
/// layout or resize.
pub type UiScrollOffsets = BTreeMap<ElementId, UiScrollState>;

/// Measured vertical extents for one visible scroll viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct UiScrollMetrics {
    id: ElementId,
    viewport_height: f32,
    content_height: f32,
}

impl UiScrollMetrics {
    /// Returns the scroll viewport's stable element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the visible viewport height in logical pixels.
    #[must_use]
    pub const fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Returns the child content height in logical pixels.
    #[must_use]
    pub const fn content_height(&self) -> f32 {
        self.content_height
    }
}

/// One semantic element in source paint order.
///
/// A clipped element stays in the layout with empty [`Self::bounds`], so an
/// accessibility adapter can preserve bounded document navigation without
/// making that element hit-testable or focusable.
#[derive(Clone, Debug, PartialEq)]
pub struct UiLayoutItem {
    id: ElementId,
    bounds: UiRect,
    paint_bounds: UiRect,
    kind: UiLayoutKind,
    enabled: bool,
}

impl UiLayoutItem {
    /// Returns the element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the clipped visible bounds.
    #[must_use]
    pub const fn bounds(&self) -> UiRect {
        self.bounds
    }

    /// Returns the element's logical bounds before ancestor clipping.
    ///
    /// A renderer uses these coordinates to preserve the element's geometry,
    /// then clips its output to [`Self::bounds`]. Input, focus, and
    /// accessibility must use only the visible bounds.
    #[must_use]
    pub const fn paint_bounds(&self) -> UiRect {
        self.paint_bounds
    }

    /// Returns the element kind.
    #[must_use]
    pub const fn kind(&self) -> UiLayoutKind {
        self.kind
    }

    /// Returns whether an action can be hit tested.
    ///
    /// Non-action items always return `false`.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// The only event produced by UI hit testing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEvent {
    /// An enabled visible action was invoked by its semantic element ID.
    ActionInvoked(ElementId),
}

impl UiEvent {
    /// Returns the action ID carried by this semantic event.
    #[must_use]
    pub fn action_id(&self) -> &ElementId {
        match self {
            Self::ActionInvoked(id) => id,
        }
    }
}

/// The bounded result of a deterministic document layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiLayout {
    items: Vec<UiLayoutItem>,
    scroll_metrics: Vec<UiScrollMetrics>,
}

impl UiLayout {
    /// Returns every semantic item in source paint order.
    ///
    /// An item's [`UiLayoutItem::bounds`] can be empty when ancestor clipping
    /// currently hides it. Input and rendering use that clipped rectangle;
    /// accessibility navigation can retain the bounded semantic record.
    #[must_use]
    pub fn items(&self) -> &[UiLayoutItem] {
        &self.items
    }

    /// Returns laid-out scroll viewport extents in source order.
    ///
    /// A host retains each viewport's [`UiScrollState`] separately, then uses
    /// these extents to clamp it after a layout or resize.
    #[must_use]
    pub fn scroll_metrics(&self) -> &[UiScrollMetrics] {
        &self.scroll_metrics
    }

    /// Returns clipped visible bounds for an element ID, if visible.
    #[must_use]
    pub fn bounds(&self, id: &ElementId) -> Option<UiRect> {
        self.items
            .iter()
            .find(|item| item.id == *id)
            .map(UiLayoutItem::bounds)
            .filter(|bounds| !bounds.is_empty())
    }

    /// Returns the focusable item at a point, in reverse paint order.
    ///
    /// Wider than [`hit_test`](Self::hit_test): that answers "what action did
    /// this activate", and this answers "what should now have focus". A field
    /// is reachable here and not there, because clicking one must put the caret
    /// in it without producing a semantic event.
    #[must_use]
    pub fn focus_target_at(&self, point: UiPoint) -> Option<&ElementId> {
        self.items
            .iter()
            .rev()
            .find(|item| {
                matches!(item.kind, UiLayoutKind::Action | UiLayoutKind::Field)
                    && item.enabled
                    && item.bounds.contains(point)
            })
            .map(|item| &item.id)
    }

    /// Hit tests enabled visible actions in reverse paint order.
    ///
    /// The result carries no command or native authority. Its receiver must
    /// apply its own session, permission, and lifecycle rules.
    #[must_use]
    pub fn hit_test(&self, point: UiPoint) -> Option<UiEvent> {
        self.items
            .iter()
            .rev()
            .find(|item| {
                item.kind == UiLayoutKind::Action && item.enabled && item.bounds.contains(point)
            })
            .map(|item| UiEvent::ActionInvoked(item.id.clone()))
    }
}

impl UiDocument {
    /// Lays out this validated document within a host client rectangle.
    ///
    /// Stacks and actions stretch across the available cross axis. Text wraps
    /// to that available width and takes the measured size of the lines it
    /// becomes, so a run that reflowed moves what follows it rather than
    /// overlapping it. Every item retains its semantic record while its
    /// clipped bounds describe the part visible through each ancestor stack
    /// and the client rectangle. An empty or non-finite client rectangle
    /// produces no items.
    #[must_use]
    pub fn layout(&self, client_bounds: UiRect, measurer: &dyn TextMeasurer) -> UiLayout {
        self.layout_with_scroll_offsets(client_bounds, measurer, &UiScrollOffsets::new())
    }

    /// Lays out this document with host-owned vertical scroll positions.
    ///
    /// Missing viewport IDs start at offset zero. The supplied positions are
    /// never changed. Each visible scroll child is vertically translated by
    /// its current clamped position and clipped to its viewport on all edges.
    /// [`UiLayout::scroll_metrics`] reports the extents that a host needs to
    /// retain a clamped position for the next pass.
    #[must_use]
    pub fn layout_with_scroll_offsets(
        &self,
        client_bounds: UiRect,
        measurer: &dyn TextMeasurer,
        scroll_offsets: &UiScrollOffsets,
    ) -> UiLayout {
        if client_bounds.is_empty() {
            return UiLayout::default();
        }
        let root_bounds = root_bounds(self.root(), client_bounds, measurer);
        let mut layout = UiLayout::default();
        layout_node(
            self.root(),
            root_bounds,
            client_bounds,
            measurer,
            scroll_offsets,
            &mut layout,
        );
        layout
    }
}

fn root_bounds(node: &UiNode, client_bounds: UiRect, measurer: &dyn TextMeasurer) -> UiRect {
    match node {
        UiNode::Text(text) => {
            bounded_text_bounds(text.value(), text.font_size(), client_bounds, measurer)
        }
        UiNode::Status(status) => {
            bounded_text_bounds(status.value(), status.font_size(), client_bounds, measurer)
        }
        UiNode::Stack(_) | UiNode::Scroll(_) | UiNode::Action(_) | UiNode::Field(_) => {
            client_bounds
        }
    }
}

fn layout_node(
    node: &UiNode,
    bounds: UiRect,
    clip: UiRect,
    measurer: &dyn TextMeasurer,
    scroll_offsets: &UiScrollOffsets,
    layout: &mut UiLayout,
) {
    let visible_bounds = bounds.intersect(clip);

    match node {
        UiNode::Stack(stack) => {
            layout.items.push(UiLayoutItem {
                id: stack.id.clone(),
                bounds: visible_bounds,
                paint_bounds: bounds,
                kind: UiLayoutKind::Stack,
                enabled: false,
            });
            layout_stack_children(stack, bounds, clip, measurer, scroll_offsets, layout);
        }
        UiNode::Scroll(scroll) => {
            layout.items.push(UiLayoutItem {
                id: scroll.id.clone(),
                bounds: visible_bounds,
                paint_bounds: bounds,
                kind: UiLayoutKind::Scroll,
                enabled: false,
            });
            layout_scroll_child(
                scroll,
                bounds,
                visible_bounds,
                measurer,
                scroll_offsets,
                layout,
            );
        }
        UiNode::Text(text) => layout.items.push(UiLayoutItem {
            id: text.id.clone(),
            bounds: visible_bounds,
            paint_bounds: bounds,
            kind: UiLayoutKind::Text,
            enabled: false,
        }),
        UiNode::Status(status) => layout.items.push(UiLayoutItem {
            id: status.id.clone(),
            bounds: visible_bounds,
            paint_bounds: bounds,
            kind: UiLayoutKind::Status,
            enabled: false,
        }),
        UiNode::Action(action) => layout.items.push(UiLayoutItem {
            id: action.id.clone(),
            bounds: visible_bounds,
            paint_bounds: bounds,
            kind: UiLayoutKind::Action,
            enabled: action.enabled,
        }),
        UiNode::Field(field) => layout.items.push(UiLayoutItem {
            id: field.id.clone(),
            bounds: visible_bounds,
            paint_bounds: bounds,
            kind: UiLayoutKind::Field,
            enabled: field.enabled,
        }),
    }
}

fn layout_stack_children(
    stack: &Stack,
    bounds: UiRect,
    clip: UiRect,
    measurer: &dyn TextMeasurer,
    scroll_offsets: &UiScrollOffsets,
    layout: &mut UiLayout,
) {
    let content = bounds.inset(stack.padding);
    let child_clip = clip.intersect(content);
    if content.is_empty() {
        return;
    }

    let gap = f32::from(stack.gap);
    match stack.axis {
        Axis::Vertical => {
            let mut cursor = content.top;
            for child in &stack.children {
                // The column a child wraps in is the stack's content width, and
                // it is the same width the child is then laid out in.
                let intrinsic = intrinsic_size(child, content.width(), measurer);
                let width = match child {
                    UiNode::Text(_) | UiNode::Status(_) => intrinsic.width.min(content.width()),
                    UiNode::Stack(_) | UiNode::Scroll(_) | UiNode::Action(_) | UiNode::Field(_) => {
                        content.width()
                    }
                };
                let child_bounds =
                    UiRect::from_size(content.left, cursor, width.max(0.0), intrinsic.height);
                layout_node(
                    child,
                    child_bounds,
                    child_clip,
                    measurer,
                    scroll_offsets,
                    layout,
                );
                cursor += intrinsic.height + gap;
            }
        }
        Axis::Horizontal => {
            let mut cursor = content.left;
            for child in &stack.children {
                // A child of a horizontal stack wraps against the width still
                // unused on the row, so a run late in the row does not measure
                // itself against space its siblings already took.
                let remaining = (content.right - cursor).max(0.0);
                let intrinsic = intrinsic_size(child, remaining, measurer);
                let height = match child {
                    UiNode::Text(_) | UiNode::Status(_) => intrinsic.height.min(content.height()),
                    UiNode::Stack(_) | UiNode::Scroll(_) | UiNode::Action(_) | UiNode::Field(_) => {
                        content.height()
                    }
                };
                let child_bounds =
                    UiRect::from_size(cursor, content.top, intrinsic.width, height.max(0.0));
                layout_node(
                    child,
                    child_bounds,
                    child_clip,
                    measurer,
                    scroll_offsets,
                    layout,
                );
                cursor += intrinsic.width + gap;
            }
        }
    }
}

fn layout_scroll_child(
    scroll: &Scroll,
    viewport_bounds: UiRect,
    viewport_clip: UiRect,
    measurer: &dyn TextMeasurer,
    scroll_offsets: &UiScrollOffsets,
    layout: &mut UiLayout,
) {
    // A scroll viewport never widens for its content, so its child wraps
    // against the viewport width. Content taller than the viewport is the
    // point; content wider than it would be unreachable.
    let content_height = intrinsic_size(scroll.child(), viewport_bounds.width(), measurer)
        .height
        .max(viewport_bounds.height());
    let mut state = scroll_offsets.get(scroll.id()).copied().unwrap_or_default();
    state.clamp(viewport_bounds.height(), content_height);
    layout.scroll_metrics.push(UiScrollMetrics {
        id: scroll.id().clone(),
        viewport_height: viewport_bounds.height(),
        content_height,
    });
    let child_bounds = UiRect::from_size(
        viewport_bounds.left,
        viewport_bounds.top + state.content_translation_y(),
        viewport_bounds.width(),
        content_height,
    );
    layout_node(
        scroll.child(),
        child_bounds,
        viewport_clip,
        measurer,
        scroll_offsets,
        layout,
    );
}

#[cfg(test)]
mod tests;
