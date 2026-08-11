//! Deterministic UI layout and semantic action hit testing.

use std::collections::BTreeMap;

use crate::{
    Action, Axis, ElementId, Field, Scroll, Stack, Text, UiDocument, UiNode, UiPoint, UiRect,
    UiScrollState, UiSize,
};

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

/// One visible element in source paint order.
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

/// The visible result of a deterministic document layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiLayout {
    items: Vec<UiLayoutItem>,
    scroll_metrics: Vec<UiScrollMetrics>,
}

impl UiLayout {
    /// Returns visible items in source paint order.
    #[must_use]
    pub fn items(&self) -> &[UiLayoutItem] {
        &self.items
    }

    /// Returns visible scroll viewport extents in source order.
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
    /// Stacks and actions stretch across the available cross axis. Text uses
    /// its measured size, bounded by that available area. Every visible output
    /// item is clipped to each ancestor stack content rectangle and the client
    /// rectangle. An empty or non-finite client rectangle produces no items.
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
        UiNode::Text(text) => bounded_text_bounds(text, client_bounds, measurer),
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
    if visible_bounds.is_empty() {
        return;
    }

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
    if content.is_empty() || child_clip.is_empty() {
        return;
    }

    let gap = f32::from(stack.gap);
    match stack.axis {
        Axis::Vertical => {
            let mut cursor = content.top;
            for child in &stack.children {
                let intrinsic = intrinsic_size(child, measurer);
                let width = match child {
                    UiNode::Text(_) => intrinsic.width.min(content.width()),
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
                let intrinsic = intrinsic_size(child, measurer);
                let height = match child {
                    UiNode::Text(_) => intrinsic.height.min(content.height()),
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
    let content_height = intrinsic_size(scroll.child(), measurer)
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

fn intrinsic_size(node: &UiNode, measurer: &dyn TextMeasurer) -> UiSize {
    match node {
        UiNode::Text(text) => measured_text(text.value(), text.font_size(), measurer),
        UiNode::Action(action) => intrinsic_action_size(action, measurer),
        UiNode::Field(field) => intrinsic_field_size(field, measurer),
        UiNode::Stack(stack) => intrinsic_stack_size(stack, measurer),
        UiNode::Scroll(scroll) => intrinsic_size(scroll.child(), measurer),
    }
}

fn measured_text(value: &str, font_size: u16, measurer: &dyn TextMeasurer) -> UiSize {
    measurer.measure(value, font_size).sanitized()
}

fn intrinsic_action_size(action: &Action, measurer: &dyn TextMeasurer) -> UiSize {
    let label = measured_text(action.label(), action.font_size(), measurer);
    UiSize::new(
        label.width + ACTION_HORIZONTAL_PADDING * 2.0,
        (label.height + ACTION_VERTICAL_PADDING * 2.0).max(ACTION_MINIMUM_HEIGHT),
    )
}

/// A field's height comes from its font, never from its current text.
///
/// Sizing to the value would make a field grow and shrink as someone types,
/// moving every sibling under their cursor. It would also leak the length of
/// what is being typed into the layout, which is the sort of thing an
/// application can observe; a field that is the same size empty or full tells
/// nobody anything. Width is taken from the stack, like an action's.
fn intrinsic_field_size(field: &Field, measurer: &dyn TextMeasurer) -> UiSize {
    // Measured from the label rather than the value, so the height reflects the
    // font in use and not what has been entered.
    let text = measured_text(field.label(), field.font_size(), measurer);
    UiSize::new(
        text.width + FIELD_HORIZONTAL_PADDING * 2.0,
        (text.height + FIELD_VERTICAL_PADDING * 2.0).max(FIELD_MINIMUM_HEIGHT),
    )
}

fn intrinsic_stack_size(stack: &Stack, measurer: &dyn TextMeasurer) -> UiSize {
    let mut primary: f32 = 0.0;
    let mut cross: f32 = 0.0;
    for child in &stack.children {
        let child_size = intrinsic_size(child, measurer);
        let (child_primary, child_cross) = match stack.axis {
            Axis::Vertical => (child_size.height, child_size.width),
            Axis::Horizontal => (child_size.width, child_size.height),
        };
        if primary > 0.0 {
            primary += f32::from(stack.gap);
        }
        primary += child_primary;
        cross = cross.max(child_cross);
    }

    let horizontal_padding = f32::from(stack.padding.left + stack.padding.right);
    let vertical_padding = f32::from(stack.padding.top + stack.padding.bottom);
    match stack.axis {
        Axis::Vertical => UiSize::new(cross + horizontal_padding, primary + vertical_padding),
        Axis::Horizontal => UiSize::new(primary + horizontal_padding, cross + vertical_padding),
    }
}

fn bounded_text_bounds(text: &Text, client_bounds: UiRect, measurer: &dyn TextMeasurer) -> UiRect {
    let size = measured_text(text.value(), text.font_size(), measurer);
    UiRect::from_size(
        client_bounds.left,
        client_bounds.top,
        size.width.min(client_bounds.width()).max(0.0),
        size.height.min(client_bounds.height()).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Axis, ElementId, Insets, Scroll, Stack, Text, UiError, UiScrollState};

    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, font_size: u16) -> UiSize {
            UiSize::new(text.len() as f32 * 10.0, f32::from(font_size))
        }
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn text(id_value: &str, value: &str) -> UiNode {
        UiNode::Text(Text::new(id(id_value), value, 10).expect("test text is valid"))
    }

    fn action(id_value: &str, label: &str, enabled: bool) -> UiNode {
        UiNode::Action(Action::new(id(id_value), label, 10, enabled).expect("test action is valid"))
    }

    fn stack(
        id_value: &str,
        axis: Axis,
        padding: Insets,
        gap: u16,
        children: Vec<UiNode>,
    ) -> UiNode {
        UiNode::Stack(
            Stack::new(id(id_value), axis, padding, gap, children).expect("test stack is valid"),
        )
    }

    fn scroll(id_value: &str, child: UiNode) -> UiNode {
        UiNode::Scroll(Scroll::new(id(id_value), child))
    }

    #[test]
    fn rejects_duplicate_document_ids() {
        let root = stack(
            "root",
            Axis::Vertical,
            Insets::zero(),
            0,
            vec![text("same", "first"), text("same", "second")],
        );
        assert_eq!(UiDocument::new(root), Err(UiError::DuplicateElementId));
    }

    #[test]
    fn rejects_invalid_model_values() {
        assert_eq!(ElementId::new("-leading"), Err(UiError::InvalidElementId));
        assert_eq!(ElementId::new("has space"), Err(UiError::InvalidElementId));
        assert_eq!(
            Text::new(id("label"), "line\nbreak", 10),
            Err(UiError::InvalidText)
        );
        assert_eq!(
            Action::new(id("open"), "Open", 7, true),
            Err(UiError::InvalidFontSize)
        );
        assert_eq!(Insets::all(257), Err(UiError::InvalidSpacing));
    }

    #[test]
    fn lays_out_vertical_stacks_in_source_order() {
        let root = stack(
            "root",
            Axis::Vertical,
            Insets::all(2).expect("padding is valid"),
            3,
            vec![text("title", "a"), action("continue", "go", true)],
        );
        let document = UiDocument::new(root).expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 90.0), &FixedMeasurer);

        assert_eq!(
            layout.bounds(&id("title")),
            Some(UiRect::new(2.0, 2.0, 12.0, 12.0))
        );
        assert_eq!(
            layout.bounds(&id("continue")),
            Some(UiRect::new(2.0, 15.0, 98.0, 51.0))
        );
    }

    #[test]
    fn lays_out_horizontal_stacks_in_source_order() {
        let root = stack(
            "root",
            Axis::Horizontal,
            Insets::zero(),
            2,
            vec![text("title", "a"), action("continue", "go", true)],
        );
        let document = UiDocument::new(root).expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 50.0), &FixedMeasurer);

        assert_eq!(
            layout.bounds(&id("title")),
            Some(UiRect::new(0.0, 0.0, 10.0, 10.0))
        );
        assert_eq!(
            layout.bounds(&id("continue")),
            Some(UiRect::new(12.0, 0.0, 64.0, 50.0))
        );
    }

    #[test]
    fn clips_actions_to_every_ancestor_content_rectangle() {
        let inner = stack(
            "inner",
            Axis::Vertical,
            Insets::all(10).expect("padding is valid"),
            0,
            vec![action("open", "Open", true)],
        );
        let root = stack("root", Axis::Vertical, Insets::zero(), 0, vec![inner]);
        let document = UiDocument::new(root).expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 50.0, 50.0), &FixedMeasurer);

        assert_eq!(
            layout.bounds(&id("open")),
            Some(UiRect::new(10.0, 10.0, 40.0, 46.0))
        );
        assert_eq!(layout.hit_test(UiPoint::new(5.0, 20.0)), None);
        assert_eq!(
            layout.hit_test(UiPoint::new(20.0, 20.0)),
            Some(UiEvent::ActionInvoked(id("open")))
        );
    }

    #[test]
    fn excludes_disabled_actions_from_hit_testing() {
        let document =
            UiDocument::new(action("disabled", "Disabled", false)).expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 50.0), &FixedMeasurer);
        assert_eq!(layout.hit_test(UiPoint::new(20.0, 20.0)), None);
    }

    #[test]
    fn returns_the_top_most_action_in_reverse_paint_order() {
        let lower = UiLayoutItem {
            id: id("lower"),
            bounds: UiRect::from_size(0.0, 0.0, 50.0, 50.0),
            paint_bounds: UiRect::from_size(0.0, 0.0, 50.0, 50.0),
            kind: UiLayoutKind::Action,
            enabled: true,
        };
        let upper = UiLayoutItem {
            id: id("upper"),
            bounds: UiRect::from_size(0.0, 0.0, 50.0, 50.0),
            paint_bounds: UiRect::from_size(0.0, 0.0, 50.0, 50.0),
            kind: UiLayoutKind::Action,
            enabled: true,
        };
        let layout = UiLayout {
            items: vec![lower, upper],
            scroll_metrics: vec![],
        };
        assert_eq!(
            layout.hit_test(UiPoint::new(25.0, 25.0)),
            Some(UiEvent::ActionInvoked(id("upper")))
        );
    }

    #[test]
    fn actions_stretch_across_the_available_cross_axis() {
        let root = stack(
            "root",
            Axis::Vertical,
            Insets::all(5).expect("padding is valid"),
            0,
            vec![action("continue", "Continue", true)],
        );
        let document = UiDocument::new(root).expect("document is valid");
        let narrow = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 60.0), &FixedMeasurer);
        let wide = document.layout(UiRect::from_size(0.0, 0.0, 180.0, 60.0), &FixedMeasurer);

        assert_eq!(
            narrow.bounds(&id("continue")),
            Some(UiRect::new(5.0, 5.0, 95.0, 41.0))
        );
        assert_eq!(
            wide.bounds(&id("continue")),
            Some(UiRect::new(5.0, 5.0, 175.0, 41.0))
        );
    }

    #[test]
    fn ignores_invalid_text_measurements() {
        struct InvalidMeasurer;
        impl TextMeasurer for InvalidMeasurer {
            fn measure(&self, _: &str, _: u16) -> UiSize {
                UiSize::new(f32::NAN, -1.0)
            }
        }

        let document = UiDocument::new(text("title", "Visible")).expect("document is valid");
        assert!(
            document
                .layout(UiRect::from_size(0.0, 0.0, 100.0, 100.0), &InvalidMeasurer)
                .items()
                .is_empty()
        );
    }

    #[test]
    fn translates_scroll_content_and_clips_actions_to_its_viewport() {
        let document = UiDocument::new(scroll(
            "viewport",
            stack(
                "content",
                Axis::Vertical,
                Insets::zero(),
                0,
                vec![
                    action("first", "First", true),
                    action("second", "Second", true),
                    action("third", "Third", true),
                ],
            ),
        ))
        .expect("document is valid");
        let mut offsets = UiScrollOffsets::new();
        let mut state = UiScrollState::default();
        assert!(state.scroll_to(36.0, 60.0, 108.0));
        offsets.insert(id("viewport"), state);

        let layout = document.layout_with_scroll_offsets(
            UiRect::from_size(0.0, 0.0, 100.0, 60.0),
            &FixedMeasurer,
            &offsets,
        );

        assert_eq!(
            layout.scroll_metrics(),
            &[UiScrollMetrics {
                id: id("viewport"),
                viewport_height: 60.0,
                content_height: 108.0,
            }]
        );
        assert_eq!(layout.bounds(&id("first")), None);
        assert_eq!(
            layout
                .items()
                .iter()
                .find(|item| item.id() == &id("second"))
                .expect("second action is visible")
                .paint_bounds(),
            UiRect::new(0.0, 0.0, 100.0, 36.0)
        );
        assert_eq!(
            layout.bounds(&id("second")),
            Some(UiRect::new(0.0, 0.0, 100.0, 36.0))
        );
        assert_eq!(
            layout.bounds(&id("third")),
            Some(UiRect::new(0.0, 36.0, 100.0, 60.0))
        );
        assert_eq!(
            layout.hit_test(UiPoint::new(10.0, 10.0)),
            Some(UiEvent::ActionInvoked(id("second")))
        );
        assert_eq!(
            layout.hit_test(UiPoint::new(10.0, 50.0)),
            Some(UiEvent::ActionInvoked(id("third")))
        );
    }

    #[test]
    fn clamps_stale_scroll_input_without_mutating_host_state() {
        let document = UiDocument::new(scroll(
            "viewport",
            stack(
                "content",
                Axis::Vertical,
                Insets::zero(),
                0,
                vec![
                    action("first", "First", true),
                    action("second", "Second", true),
                ],
            ),
        ))
        .expect("document is valid");
        let mut offsets = UiScrollOffsets::new();
        let mut stale_state = UiScrollState::default();
        assert!(stale_state.scroll_to(300.0, 0.0, 300.0));
        offsets.insert(id("viewport"), stale_state);

        let layout = document.layout_with_scroll_offsets(
            UiRect::from_size(0.0, 0.0, 100.0, 60.0),
            &FixedMeasurer,
            &offsets,
        );

        assert_eq!(offsets[&id("viewport")].offset_y(), 300.0);
        assert_eq!(
            layout.bounds(&id("first")),
            Some(UiRect::new(0.0, 0.0, 100.0, 24.0))
        );
        assert_eq!(
            layout.bounds(&id("second")),
            Some(UiRect::new(0.0, 24.0, 100.0, 60.0))
        );
    }
}
