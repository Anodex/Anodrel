//! Field and action control painting for one retained UI-Lab view.

use super::*;

/// Draws one field's box, its current host-owned text, and the caret.
///
/// The text comes from the host's field state, not from the document: the
/// document only ever seeded it. See `docs/UI_FIELDS.md`.
pub(super) fn draw_field(
    canvas: &mut Canvas,
    lab: &UiLab,
    field: &Field,
    bounds: UiRect,
    surface: Surface,
    palette: UiLabPalette,
) {
    let box_bounds = surface.to_canvas_rect(bounds);
    let radius = 8.0 * surface.scale;
    let focused = lab.focus.focused() == Some(field.id());
    canvas.fill_rounded_rect(box_bounds, radius, &Paint::solid(palette.backdrop_lift));
    canvas.stroke_rounded_rect(
        box_bounds,
        radius,
        if focused { 2.0 } else { 1.0 } * surface.scale,
        &Paint::solid(if focused {
            palette.accent_ipc
        } else {
            palette.panel_edge
        }),
    );

    let state = lab.fields.get(field.id());
    let entered = state.map_or("", UiFieldState::text);
    // The placeholder stands in only while there is nothing to show, and is
    // drawn in the dimmer ink so it never reads as entered text.
    let (value, color) = if entered.is_empty() {
        (
            field.placeholder().unwrap_or(""),
            if field.enabled() {
                palette.ink_soft
            } else {
                palette.panel_edge
            },
        )
    } else {
        (
            entered,
            if field.enabled() {
                palette.ink
            } else {
                palette.ink_soft
            },
        )
    };

    let inset = FIELD_HORIZONTAL_PADDING * surface.scale;
    let left = box_bounds.left + inset;
    let font = surface.font(field.font_size());
    let spec = TextSpec::new(value, font, WEIGHT_REGULAR);
    let baseline = box_bounds.top + (box_bounds.height() - text::line_height(&spec)) / 2.0;
    if !value.is_empty() {
        text::draw(
            canvas,
            &spec,
            point(left, baseline),
            Align::Left,
            &Paint::solid(color),
        );
    }

    if !focused {
        return;
    }
    // The caret is placed by measuring the text before it, so it lands between
    // characters at any font and never inside one.
    let Some(state) = state else {
        return;
    };
    let before = TextSpec::new(&entered[..state.caret()], font, WEIGHT_REGULAR);
    let caret_x = left + text::width(&before);
    let caret_height = text::line_height(&spec);
    canvas.fill_rect(
        Rect::new(
            caret_x,
            baseline,
            caret_x + (1.0 * surface.scale).max(1.0),
            baseline + caret_height,
        ),
        &Paint::solid(palette.ink),
    );
}

pub(super) fn draw_action(
    canvas: &mut Canvas,
    lab: &UiLab,
    action: &Action,
    bounds: UiRect,
    surface: Surface,
    palette: UiLabPalette,
) {
    let bounds = surface.to_canvas_rect(bounds);
    let hovered = lab.hovered.as_ref() == Some(action.id());
    let focused = lab.focus.focused() == Some(action.id());
    let fill = match (action.tone(), hovered) {
        (UiActionTone::Accent, true) => palette.accent_core,
        (UiActionTone::Accent, false) => palette.accent_shell,
        (UiActionTone::Neutral, true) => palette.panel_raised,
        (UiActionTone::Neutral, false) => palette.backdrop_lift,
    };
    let edge = if focused {
        palette.accent_ipc
    } else if action.tone() == UiActionTone::Accent || hovered {
        palette.accent_shell
    } else {
        palette.panel_edge
    };
    canvas.fill_rounded_rect(bounds, 10.0 * surface.scale, &Paint::solid(fill));
    canvas.stroke_rounded_rect(
        bounds,
        10.0 * surface.scale,
        if focused { 2.0 } else { 1.0 } * surface.scale,
        &Paint::solid(edge),
    );

    let spec = TextSpec::new(
        action.label(),
        surface.font(action.font_size()),
        WEIGHT_REGULAR,
    );
    let baseline = bounds.top + (bounds.height() - text::line_height(&spec)) / 2.0;
    text::draw(
        canvas,
        &spec,
        point((bounds.left + bounds.right) / 2.0, baseline),
        Align::Center,
        &Paint::solid(if action.tone() == UiActionTone::Accent {
            palette.accent_text
        } else {
            palette.button_text
        }),
    );
}
