//! Intrinsic measurement helpers for deterministic document layout.

use super::*;

/// Measures a node against the width it will actually be laid out in.
///
/// Text is the only kind whose height depends on that width, because it is the
/// only one that wraps. An action or a field measures its label on one line
/// whatever the column: a control that reflowed into a paragraph would stop
/// looking like a control.
pub(super) fn intrinsic_size(
    node: &UiNode,
    available_width: f32,
    measurer: &dyn TextMeasurer,
) -> UiSize {
    match node {
        UiNode::Text(text) => {
            measured_wrapped_text(text.value(), text.font_size(), available_width, measurer)
        }
        UiNode::Status(status) => measured_wrapped_text(
            status.value(),
            status.font_size(),
            available_width,
            measurer,
        ),
        UiNode::Action(action) => intrinsic_action_size(action, measurer),
        UiNode::Field(field) => intrinsic_field_size(field, measurer),
        UiNode::Stack(stack) => intrinsic_stack_size(stack, available_width, measurer),
        UiNode::Scroll(scroll) => intrinsic_size(scroll.child(), available_width, measurer),
    }
}

fn measured_text(value: &str, font_size: u16, measurer: &dyn TextMeasurer) -> UiSize {
    measurer.measure(value, font_size).sanitized()
}

/// Measures one text value as the block of lines it occupies at this width.
///
/// The reported width is the **widest line**, not the width it was wrapped
/// against, so a short run still reports its natural size and a stack's cross
/// axis does not inflate to the column. Greedy breaking makes that safe: see
/// the stability test in [`crate::wrap_text`]'s module, which pins that
/// re-wrapping at the widest line reproduces the same lines — which is what a
/// host relies on when it paints from the bounds it was given.
fn measured_wrapped_text(
    value: &str,
    font_size: u16,
    available_width: f32,
    measurer: &dyn TextMeasurer,
) -> UiSize {
    let line_height = measured_text(value, font_size, measurer).height;
    let lines = wrap_text(value, font_size, available_width, measurer);
    let width = lines
        .iter()
        .map(|line| measured_text(line, font_size, measurer).width)
        .fold(0.0_f32, f32::max);
    UiSize::new(width, wrapped_height(lines.len(), line_height))
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

fn intrinsic_stack_size(
    stack: &Stack,
    available_width: f32,
    measurer: &dyn TextMeasurer,
) -> UiSize {
    let horizontal_padding = f32::from(stack.padding.left + stack.padding.right);
    // Children are measured against the column left after this stack's own
    // padding, which is the width they will be laid out in.
    let child_width = (available_width - horizontal_padding).max(0.0);
    let mut primary: f32 = 0.0;
    let mut cross: f32 = 0.0;
    for child in &stack.children {
        let child_size = intrinsic_size(child, child_width, measurer);
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

    let vertical_padding = f32::from(stack.padding.top + stack.padding.bottom);
    match stack.axis {
        Axis::Vertical => UiSize::new(cross + horizontal_padding, primary + vertical_padding),
        Axis::Horizontal => UiSize::new(primary + horizontal_padding, cross + vertical_padding),
    }
}

pub(super) fn bounded_text_bounds(
    value: &str,
    font_size: u16,
    client_bounds: UiRect,
    measurer: &dyn TextMeasurer,
) -> UiRect {
    let size = measured_wrapped_text(value, font_size, client_bounds.width(), measurer);
    UiRect::from_size(
        client_bounds.left,
        client_bounds.top,
        size.width.min(client_bounds.width()).max(0.0),
        size.height.min(client_bounds.height()).max(0.0),
    )
}
