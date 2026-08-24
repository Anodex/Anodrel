use super::super::accessibility::scroll_into_view_offset;
use super::super::*;
use super::id;

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
