//! Focused verification for the retained Windows UI Lab.

use super::accessibility::scroll_into_view_offset;
use super::*;
use anodrel_ui::UiAccessibilityRole;

fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn id(value: &str) -> ElementId {
    ElementId::new(value).expect("fixed UI Lab ID is valid")
}

/// Tabs until the sample field has focus, then returns the lab.
fn focused_on_the_field() -> UiLab {
    let mut lab = UiLab::new();
    for _ in 0..8 {
        if lab.focus.focused() == Some(&id("ui.lab.field")) {
            return lab;
        }
        lab.focus_next(BASE_WIDTH, BASE_HEIGHT);
    }
    panic!("focus never reached the sample field");
}

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

#[test]
fn visual_hierarchy_comes_from_semantic_roles_not_element_names() {
    let lab = UiLab::new();
    let UiNode::Scroll(viewport) = lab.document.root() else {
        panic!("fixed UI Lab root is a scroll viewport");
    };
    let UiNode::Stack(content) = viewport.child() else {
        panic!("fixed UI Lab viewport has a content stack");
    };
    let UiNode::Stack(root) = &content.children()[0] else {
        panic!("fixed UI Lab fixture is a stack");
    };

    let eyebrow = match &root.children()[0] {
        UiNode::Text(text) => text,
        _ => panic!("fixed UI Lab eyebrow is text"),
    };
    let detail = match &root.children()[2] {
        UiNode::Text(text) => text,
        _ => panic!("fixed UI Lab detail is text"),
    };
    let UiNode::Stack(actions) = &root.children()[3] else {
        panic!("fixed UI Lab actions are a stack");
    };
    // Found by ID rather than by position: this document gains nodes over
    // time, and an index would keep silently pointing at a different one.
    let emphasized_action = actions
        .children()
        .iter()
        .find_map(|child| match child {
            UiNode::Action(action) if action.id() == &id("ui.lab.hit-test") => Some(action),
            _ => None,
        })
        .expect("fixed UI Lab emphasized action exists");

    assert_eq!(eyebrow.tone(), UiTextTone::Accent);
    assert_eq!(detail.tone(), UiTextTone::Secondary);
    assert_eq!(actions.surface_tone(), UiSurfaceTone::Raised);
    assert_eq!(emphasized_action.tone(), UiActionTone::Accent);
}

#[test]
fn page_scrolling_changes_only_the_lab_owned_viewport_position() {
    let mut lab = UiLab::new();

    assert!(lab.scroll_page(BASE_WIDTH, BASE_HEIGHT, true));
    assert!(lab.scroll_offsets[&id("ui.lab.viewport")].offset_y() > 0.0);
    assert!(
        lab.layout(BASE_WIDTH, BASE_HEIGHT)
            .bounds(&id("ui.lab.scroll.exercise-9"))
            .is_some()
    );
}

#[test]
fn accessibility_scroll_uses_the_same_selected_retained_viewport() {
    let mut lab = UiLab::new();
    let snapshot = lab
        .accessibility_scroll_snapshot(BASE_WIDTH, BASE_HEIGHT)
        .expect("the fixed Lab viewport overflows");
    assert_eq!(snapshot.target(), &id("ui.lab.viewport"));
    assert_eq!(snapshot.vertical_scroll_percent(), 0.0);
    assert!(snapshot.vertical_view_size() > 0.0);
    assert!(snapshot.vertical_view_size() < 100.0);

    assert_eq!(
        lab.scroll_accessibility_target(
            BASE_WIDTH,
            BASE_HEIGHT,
            &id("missing"),
            UiAutomationScrollCommand::Page { forward: true },
        ),
        None,
        "a UIA request cannot select another viewport"
    );
    assert_eq!(
        lab.scroll_accessibility_target(
            BASE_WIDTH,
            BASE_HEIGHT,
            snapshot.target(),
            UiAutomationScrollCommand::Line { forward: true },
        ),
        Some(true)
    );
    assert!(lab.scroll_offsets[&id("ui.lab.viewport")].offset_y() > 0.0);

    assert_eq!(
        lab.scroll_accessibility_target(
            BASE_WIDTH,
            BASE_HEIGHT,
            snapshot.target(),
            UiAutomationScrollCommand::Percent { percent: 100.0 },
        ),
        Some(true)
    );
    let refreshed = lab
        .accessibility_scroll_snapshot(BASE_WIDTH, BASE_HEIGHT)
        .expect("the fixed Lab viewport remains overflowing");
    assert_eq!(refreshed.vertical_scroll_percent(), 100.0);
    assert_eq!(lab.last_action, None);
    assert_eq!(lab.focus.focused(), None);
}

#[test]
fn accessibility_scroll_item_reveals_an_offscreen_semantic_child() {
    let mut lab = UiLab::new();
    let snapshot = lab
        .accessibility_scroll_snapshot(BASE_WIDTH, BASE_HEIGHT)
        .expect("the fixed Lab viewport overflows");
    let target = id("ui.lab.scroll.exercise-9");
    assert!(
        lab.accessibility_scroll_items(BASE_WIDTH, BASE_HEIGHT)
            .contains(&target),
        "a bounded child of the selected viewport is published for ScrollItem"
    );
    assert!(
        lab.layout(BASE_WIDTH, BASE_HEIGHT)
            .bounds(&target)
            .is_none(),
        "the exercise starts fully clipped but stays in the semantic tree"
    );

    assert_eq!(
        lab.scroll_accessibility_target(
            BASE_WIDTH,
            BASE_HEIGHT,
            snapshot.target(),
            UiAutomationScrollCommand::ScrollIntoView {
                item: target.clone(),
            },
        ),
        Some(true)
    );
    let layout = lab.layout(BASE_WIDTH, BASE_HEIGHT);
    let item = layout
        .items()
        .iter()
        .find(|item| item.id() == &target)
        .expect("the bounded semantic child remains laid out");
    assert_eq!(layout.bounds(&target), Some(item.paint_bounds()));
    assert!(lab.scroll_offsets[&id("ui.lab.viewport")].offset_y() > 0.0);
    assert_eq!(lab.last_action, None);
    assert_eq!(lab.focus.focused(), None);
}

#[test]
fn scroll_item_geometry_uses_nearest_edge_and_never_an_alignment_option() {
    let viewport = UiRect::from_size(20.0, 40.0, 100.0, 60.0);
    assert_eq!(
        scroll_into_view_offset(viewport, UiRect::from_size(20.0, 120.0, 100.0, 20.0), 30.0,),
        Some(70.0),
        "a lower item aligns its bottom"
    );
    assert_eq!(
        scroll_into_view_offset(viewport, UiRect::from_size(20.0, 10.0, 100.0, 20.0), 30.0,),
        Some(0.0),
        "an upper item aligns its top"
    );
    assert_eq!(
        scroll_into_view_offset(viewport, UiRect::from_size(20.0, 60.0, 100.0, 100.0), 30.0,),
        Some(50.0),
        "an oversized item aligns its top"
    );
    assert_eq!(
        scroll_into_view_offset(viewport, UiRect::from_size(20.0, 50.0, 100.0, 20.0), 30.0,),
        Some(30.0),
        "a wholly visible item leaves the offset alone"
    );
    assert_eq!(
        scroll_into_view_offset(viewport, UiRect::default(), 30.0),
        None,
        "missing geometry cannot become an implicit scroll target"
    );
}

#[test]
fn scroll_item_excludes_a_nested_viewports_contents() {
    let document = anodrel_ui_document::decode_v2(
            r#"{"format":"anodrel.ui.document.v2","root":{"id":"outer","kind":"scroll","child":{"id":"outer-content","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"before","kind":"action","label":"Before","fontSize":16,"enabled":true,"tone":"accent"},{"id":"inner","kind":"scroll","child":{"id":"inner-content","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"inside","kind":"action","label":"Inside","fontSize":16,"enabled":true,"tone":"accent"}]}},{"id":"after","kind":"action","label":"After","fontSize":16,"enabled":true,"tone":"accent"}]}}}"#,
        )
        .expect("the nested scroll fixture is valid");
    let lab = UiLab::from_document_with_status(document, None);
    let ids = lab
        .accessibility_scroll_items(200.0, 40.0)
        .into_iter()
        .map(|id| id.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["outer-content", "before", "inner", "after"]);
}

#[test]
fn scrollbar_track_and_thumb_change_only_the_local_scroll_position() {
    let mut lab = UiLab::new();
    let focus_before = lab.focus.focused().cloned();
    let action_before = lab.last_action.clone();
    let (scrollbar, _) = lab
        .first_scrollbar(BASE_WIDTH, BASE_HEIGHT)
        .expect("the fixed Lab viewport overflows");
    let track_point = Point {
        x: (scrollbar.track().left + scrollbar.track().right) / 2.0,
        y: scrollbar.track().bottom - 1.0,
    };

    assert!(lab.page_scrollbar_at(BASE_WIDTH, BASE_HEIGHT, track_point));
    assert!(lab.scroll_offsets[&id("ui.lab.viewport")].offset_y() > 0.0);
    assert_eq!(lab.focus.focused().cloned(), focus_before);
    assert_eq!(lab.last_action, action_before);

    let (scrollbar, metrics) = lab
        .first_scrollbar(BASE_WIDTH, BASE_HEIGHT)
        .expect("the viewport still overflows after paging");
    let thumb = scrollbar.thumb();
    let thumb_point = Point {
        x: (thumb.left + thumb.right) / 2.0,
        y: (thumb.top + thumb.bottom) / 2.0,
    };
    assert!(lab.begin_scrollbar_drag(BASE_WIDTH, BASE_HEIGHT, thumb_point));
    assert!(lab.drag_scrollbar(
        BASE_WIDTH,
        BASE_HEIGHT,
        Point {
            x: thumb_point.x,
            y: scrollbar.track().top - 50.0,
        }
    ));
    assert!(lab.drag_scrollbar(
        BASE_WIDTH,
        BASE_HEIGHT,
        Point {
            x: thumb_point.x,
            y: scrollbar.track().bottom + 50.0,
        }
    ));
    assert!(lab.end_scrollbar_drag());
    assert_eq!(
        lab.scroll_offsets[metrics.id()].offset_y(),
        anodrel_ui::UiScrollState::maximum_offset(
            metrics.viewport_height(),
            metrics.content_height()
        )
    );
    assert_eq!(lab.focus.focused().cloned(), focus_before);
    assert_eq!(lab.last_action, action_before);
}

#[test]
fn a_document_replacement_cannot_turn_a_captured_thumb_release_into_an_action() {
    let mut lab = UiLab::new();
    let (scrollbar, _) = lab
        .first_scrollbar(BASE_WIDTH, BASE_HEIGHT)
        .expect("the fixed Lab viewport overflows");
    let thumb = scrollbar.thumb();
    assert!(lab.begin_scrollbar_drag(
        BASE_WIDTH,
        BASE_HEIGHT,
        Point {
            x: (thumb.left + thumb.right) / 2.0,
            y: (thumb.top + thumb.bottom) / 2.0,
        }
    ));

    // A session worker may replace a document while Windows still owns the
    // pointer capture. The old gesture must remain consumed until release.
    lab.replace_document(UiLab::waiting_for_session().document);
    assert!(lab.end_scrollbar_drag());
    assert!(!lab.end_scrollbar_drag());
    assert_eq!(lab.last_action, None);
}

#[test]
fn preview_documents_have_no_lab_specific_status_replacement() {
    let document = decode(
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"External text","fontSize":16,"tone":"primary"}}"#,
        )
        .expect("preview fixture is valid");
    let preview = UiLab::preview(document);

    assert!(preview.status_target.is_none());
    assert_eq!(status_text(&preview), None);
}

#[test]
fn preview_document_renders_through_the_same_native_ui_view() {
    let document = decode(
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":40,"top":40,"right":40,"bottom":40},"gap":12,"surfaceTone":"plain","children":[{"id":"title","kind":"text","value":"External preview document","fontSize":28,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#,
        )
        .expect("preview fixture is valid");
    let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    draw(&mut canvas, &UiLab::preview(document));

    let changed = (0..canvas.height())
        .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
        .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
        .count();
    assert!(changed > 1_000, "preview drew too little content");
}

#[test]
fn draws_visible_content_without_a_web_surface() {
    let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    draw(&mut canvas, &UiLab::new());
    let changed = (0..canvas.height())
        .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
        .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
        .count();
    assert!(changed > 1_000, "UI Lab drew too little content");
}
