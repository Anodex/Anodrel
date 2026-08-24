use super::super::*;
use super::{id, rgb};
use anodrel_ui::UiAccessibilityRole;

#[test]
fn high_contrast_palette_uses_only_host_supplied_system_colours() {
    let palette = UiLabPalette::high_contrast(SystemColors {
        window: rgb(1, 2, 3),
        window_text: rgb(4, 5, 6),
        button_face: rgb(7, 8, 9),
        button_text: rgb(10, 11, 12),
        highlight: rgb(13, 14, 15),
        highlight_text: rgb(16, 17, 18),
    });
    assert_eq!(palette.backdrop, Color::rgb(1, 2, 3));
    assert_eq!(palette.panel, Color::rgb(7, 8, 9));
    assert_eq!(palette.ink, Color::rgb(4, 5, 6));
    assert_eq!(palette.accent_shell, Color::rgb(13, 14, 15));
    assert_eq!(palette.accent_text, Color::rgb(16, 17, 18));
    assert_eq!(palette.button_text, Color::rgb(10, 11, 12));
    assert_eq!(palette.scrollbar_track, Color::rgb(7, 8, 9));
    assert_eq!(palette.scrollbar_thumb, Color::rgb(4, 5, 6));
}

#[test]
fn hit_testing_tracks_the_scaled_layout() {
    let lab = UiLab::new();
    let surface = Surface::new(BASE_WIDTH * 2.0, BASE_HEIGHT * 2.0);
    let layout = lab.document.layout(surface.bounds(), &WindowsTextMeasurer);
    let id = ElementId::new("ui.lab.hit-test").expect("fixed ID is valid");
    let bounds = layout.bounds(&id).expect("action is visible");
    assert_eq!(
        lab.action_at(
            BASE_WIDTH * 2.0,
            BASE_HEIGHT * 2.0,
            Point {
                x: (bounds.left + bounds.right) * surface.scale / 2.0,
                y: (bounds.top + bounds.bottom) * surface.scale / 2.0,
            },
        ),
        Some(id)
    );
}

#[test]
fn invocation_changes_only_the_host_owned_status() {
    let mut lab = UiLab::new();
    let layout = lab.document.layout(
        UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
        &WindowsTextMeasurer,
    );
    let id = ElementId::new("ui.lab.inspect").expect("fixed ID is valid");
    let bounds = layout.bounds(&id).expect("action is visible");
    assert!(lab.invoke(
        BASE_WIDTH,
        BASE_HEIGHT,
        Point {
            x: (bounds.left + bounds.right) / 2.0,
            y: (bounds.top + bounds.bottom) / 2.0,
        },
    ));
    assert_eq!(lab.last_action, Some(id));
}

#[test]
fn keyboard_focus_traverses_fields_and_actions_but_activates_only_actions() {
    // Renamed from "traverses and activates only semantic actions": Tab now
    // reaches a field too, because a person has to get to one to type. What
    // has not changed is that only an action can be activated.
    let mut lab = UiLab::new();
    assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.field")));
    assert!(
        !lab.activate_focused(BASE_WIDTH, BASE_HEIGHT),
        "a focused field was activated"
    );
    assert_eq!(lab.last_action, None);

    assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.inspect")));
    assert_eq!(lab.accessibility_focus(), Some(id("ui.lab.inspect")));
    assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.hit-test")));
    assert!(lab.activate_focused(BASE_WIDTH, BASE_HEIGHT));
    assert_eq!(lab.last_action, Some(id("ui.lab.hit-test")));
    assert!(lab.focus_previous(BASE_WIDTH, BASE_HEIGHT));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.inspect")));
}

#[test]
fn accessibility_field_values_copy_current_text_without_caret_state() {
    let mut lab = UiLab::new();
    assert!(lab.focus_next(BASE_WIDTH, BASE_HEIGHT));
    for character in "Ada".chars() {
        assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, character));
    }

    assert_eq!(
        lab.accessibility_field_values(),
        vec![(id("ui.lab.field"), "Ada".to_owned())]
    );
}

#[test]
fn exposes_the_same_button_semantics_that_the_lab_draws() {
    let lab = UiLab::new();
    let layout = lab.document.layout(
        UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
        &WindowsTextMeasurer,
    );
    let snapshot = lab.document.accessibility_snapshot(&layout);
    let buttons = snapshot
        .nodes()
        .iter()
        .filter(|node| node.role() == UiAccessibilityRole::Button)
        .map(|node| (node.id().as_str(), node.name(), node.enabled()))
        .collect::<Vec<_>>();

    assert_eq!(
        &buttons[..3],
        [
            ("ui.lab.inspect", Some("Inspect layout"), true),
            ("ui.lab.hit-test", Some("Test semantic action"), true),
            ("ui.lab.report", Some("Report semantic action"), true),
        ]
    );
}
