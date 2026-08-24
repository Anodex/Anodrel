use super::super::*;
use super::{focused_on_the_field, id};

#[test]
fn an_action_below_fields_is_still_clickable() {
    // Reproduces the sample's field document: two fields above one action.
    // A field that mis-measured its height would push the action's real
    // bounds away from where it is drawn, and the click would land nowhere.
    let json = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":56,"top":48,"right":56,"bottom":48},"gap":14,"surfaceTone":"plain","children":[{"id":"one","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true},{"id":"two","kind":"field","label":"Note","value":"edit me","maxLength":64,"fontSize":16,"enabled":true},{"id":"submit","kind":"action","label":"Submit field values","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;
    let lab = UiLab::preview(decode(json).expect("test document is valid"));

    let layout = lab.layout(BASE_WIDTH, BASE_HEIGHT);
    let bounds = layout
        .bounds(&id("submit"))
        .expect("the action is laid out and visible");
    let centre = Point {
        x: (bounds.left + bounds.right) / 2.0,
        y: (bounds.top + bounds.bottom) / 2.0,
    };
    assert_eq!(
        lab.action_at(BASE_WIDTH, BASE_HEIGHT, centre),
        Some(id("submit")),
        "the action's own centre did not hit it"
    );

    // A click on a field must not be mistaken for the action.
    let field_bounds = layout.bounds(&id("one")).expect("the field is laid out");
    assert_eq!(
        lab.action_at(
            BASE_WIDTH,
            BASE_HEIGHT,
            Point {
                x: (field_bounds.left + field_bounds.right) / 2.0,
                y: (field_bounds.top + field_bounds.bottom) / 2.0,
            }
        ),
        None
    );
}

#[test]
fn clicking_a_field_focuses_it_so_a_person_can_type_there() {
    // Without this a field was reachable only by Tab, which is not how
    // anyone expects to use a text box.
    let mut lab = UiLab::new();
    let layout = lab.layout(BASE_WIDTH, BASE_HEIGHT);
    let bounds = layout
        .bounds(&id("ui.lab.field"))
        .expect("the sample field is visible");
    let centre = Point {
        x: (bounds.left + bounds.right) / 2.0,
        y: (bounds.top + bounds.bottom) / 2.0,
    };

    assert!(lab.focus_at(BASE_WIDTH, BASE_HEIGHT, centre));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.field")));
    assert_eq!(lab.accessibility_focus(), Some(id("ui.lab.field")));
    // Focusing a field produces no semantic event, the same as tabbing to
    // one: a click that lands on a field tells an application nothing.
    assert_eq!(lab.last_action, None);
    assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'x'));

    // Clicking the same field again is not a change, so the caller does not
    // repaint for it.
    assert!(!lab.focus_at(BASE_WIDTH, BASE_HEIGHT, centre));
}

#[test]
fn automation_focus_revalidates_the_current_layout_before_it_moves() {
    let mut lab = UiLab::new();
    let field = id("ui.lab.field");
    let action = id("ui.lab.hit-test");
    let missing = id("ui.lab.missing");

    assert_eq!(
        lab.focus_accessibility_target(BASE_WIDTH, BASE_HEIGHT, &field),
        Some(true)
    );
    assert_eq!(lab.accessibility_focus(), Some(field.clone()));
    // Repeating a valid focus request is successful even though it does
    // not repaint or announce: UI Automation asked for a state that is
    // already true.
    assert_eq!(
        lab.focus_accessibility_target(BASE_WIDTH, BASE_HEIGHT, &field),
        Some(false)
    );
    assert_eq!(
        lab.focus_accessibility_target(BASE_WIDTH, BASE_HEIGHT, &action),
        Some(true)
    );
    assert_eq!(lab.accessibility_focus(), Some(action));
    assert_eq!(
        lab.focus_accessibility_target(BASE_WIDTH, BASE_HEIGHT, &missing),
        None
    );

    lab.replace_document(UiLab::waiting_for_session().document);
    assert_eq!(
        lab.focus_accessibility_target(BASE_WIDTH, BASE_HEIGHT, &field),
        None,
        "a target removed by replacement retained accessibility focus"
    );
}

#[test]
fn clicking_an_action_focuses_it_as_well_as_invoking_it() {
    let mut lab = UiLab::new();
    let layout = lab.layout(BASE_WIDTH, BASE_HEIGHT);
    let bounds = layout
        .bounds(&id("ui.lab.hit-test"))
        .expect("the action is visible");
    let centre = Point {
        x: (bounds.left + bounds.right) / 2.0,
        y: (bounds.top + bounds.bottom) / 2.0,
    };

    assert!(lab.focus_at(BASE_WIDTH, BASE_HEIGHT, centre));
    assert_eq!(lab.focus.focused(), Some(&id("ui.lab.hit-test")));
    assert!(lab.invoke(BASE_WIDTH, BASE_HEIGHT, centre));
    assert_eq!(lab.last_action, Some(id("ui.lab.hit-test")));
}

#[test]
fn clicking_empty_space_leaves_focus_where_it_was() {
    let mut lab = UiLab::new();
    lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
    let before = lab.focus.focused().cloned();
    // Well outside any node, but inside the client area.
    assert!(!lab.focus_at(BASE_WIDTH, BASE_HEIGHT, Point { x: 4.0, y: 4.0 }));
    assert_eq!(lab.focus.focused().cloned(), before);
}

#[test]
fn typing_reaches_the_focused_field_and_nothing_else() {
    let mut lab = focused_on_the_field();
    for character in "Ada".chars() {
        assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, character));
    }
    assert_eq!(
        lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
        Some("Ada")
    );

    // Typing produces no semantic event, so nothing an application could
    // ever read has changed. See Decision 0067.
    assert_eq!(lab.last_action, None);
}

#[test]
fn typing_with_an_action_focused_changes_nothing() {
    let mut lab = UiLab::new();
    lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
    while lab.focus.focused() == Some(&id("ui.lab.field")) {
        lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
    }
    assert!(!lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'x'));
    assert_eq!(
        lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
        Some("")
    );
}

#[test]
fn editing_keys_move_the_caret_and_remove_characters() {
    let mut lab = focused_on_the_field();
    for character in "abc".chars() {
        assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, character));
    }
    assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Home));
    assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Delete));
    assert_eq!(
        lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
        Some("bc")
    );
    assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::End));
    assert!(lab.edit_focused_field(BASE_WIDTH, BASE_HEIGHT, FieldEdit::Backspace));
    assert_eq!(
        lab.fields.get(&id("ui.lab.field")).map(UiFieldState::text),
        Some("b")
    );
}

#[test]
fn a_field_that_left_the_document_cannot_still_be_typed_into() {
    // The focused field is resolved against a fresh layout on every
    // keystroke, so a document replacement that removed it takes effect
    // immediately rather than at the next repaint.
    let mut lab = focused_on_the_field();
    assert!(lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'a'));

    lab.replace_document(UiLab::waiting_for_session().document);
    assert!(!lab.type_character(BASE_WIDTH, BASE_HEIGHT, 'b'));
    assert!(lab.fields.get(&id("ui.lab.field")).is_none());
}

#[test]
fn every_action_reports_its_own_semantic_id() {
    let lab = UiLab::new();
    let layout = lab.document.layout(
        UiRect::from_size(0.0, 0.0, BASE_WIDTH, BASE_HEIGHT),
        &WindowsTextMeasurer,
    );
    for expected in ["ui.lab.inspect", "ui.lab.hit-test", "ui.lab.report"] {
        let id = ElementId::new(expected).expect("fixed ID is valid");
        let bounds = layout.bounds(&id).expect("action is visible");
        assert_eq!(
            lab.action_at(
                BASE_WIDTH,
                BASE_HEIGHT,
                Point {
                    x: (bounds.left + bounds.right) / 2.0,
                    y: (bounds.top + bounds.bottom) / 2.0,
                },
            )
            .as_ref()
            .map(ElementId::as_str),
            Some(expected)
        );
    }
}
