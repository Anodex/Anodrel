//! Focused verification for deterministic native UI layout and scroll metrics.

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

fn stack(id_value: &str, axis: Axis, padding: Insets, gap: u16, children: Vec<UiNode>) -> UiNode {
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

/// The same document at two widths, which is the whole point of wrapping.
///
/// At 400 logical pixels the sentence is one line; at 100 it is two, and the
/// action beneath it moves down by exactly the line it gained. Before
/// wrapping existed the run stayed one line at both widths and the half past
/// the edge was simply cut off.
#[test]
fn a_text_run_reflows_to_its_column_and_moves_what_follows_it() {
    let root = stack(
        "root",
        Axis::Vertical,
        Insets::zero(),
        0,
        vec![text("body", "one two three four"), action("go", "Go", true)],
    );
    let document = UiDocument::new(root).expect("document is valid");

    let wide = document.layout(UiRect::from_size(0.0, 0.0, 400.0, 200.0), &FixedMeasurer);
    assert_eq!(
        wide.bounds(&id("body")),
        Some(UiRect::from_size(0.0, 0.0, 180.0, 10.0))
    );
    assert_eq!(wide.bounds(&id("go")).map(|bounds| bounds.top), Some(10.0));

    let narrow = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 200.0), &FixedMeasurer);
    assert_eq!(
        narrow.bounds(&id("body")),
        Some(UiRect::from_size(0.0, 0.0, 100.0, 20.0))
    );
    assert_eq!(
        narrow.bounds(&id("go")).map(|bounds| bounds.top),
        Some(20.0)
    );
}

#[test]
fn a_wrapped_run_stays_inside_its_stack_padding() {
    // The column a run wraps against is the content box, not the client
    // rectangle, so padding is not somewhere text may spill.
    let root = stack(
        "root",
        Axis::Vertical,
        Insets::all(20).expect("padding is valid"),
        0,
        vec![text("body", "alpha beta gamma delta epsilon")],
    );
    let document = UiDocument::new(root).expect("document is valid");
    let layout = document.layout(UiRect::from_size(0.0, 0.0, 140.0, 300.0), &FixedMeasurer);

    let bounds = layout.bounds(&id("body")).expect("the run is laid out");
    assert!(bounds.left >= 20.0, "{bounds:?} started inside the padding");
    assert!(bounds.right <= 120.0, "{bounds:?} ran past the content box");
    // Five words that measure 300 logical pixels on one line cannot be one
    // line in a 100-pixel column.
    assert!(bounds.height() > 10.0, "{bounds:?} did not wrap");
}

#[test]
fn scrollable_content_wraps_against_the_viewport_and_grows_taller() {
    // A viewport never widens for its content, so wrapping is what turns a
    // long run into something scrolling can reach.
    let root = scroll(
        "viewport",
        stack(
            "content",
            Axis::Vertical,
            Insets::zero(),
            0,
            vec![text(
                "body",
                "alpha beta gamma delta epsilon zeta eta theta",
            )],
        ),
    );
    let document = UiDocument::new(root).expect("document is valid");
    let layout = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 40.0), &FixedMeasurer);

    let metrics = layout
        .scroll_metrics()
        .iter()
        .find(|metrics| metrics.id() == &id("viewport"))
        .expect("the viewport reports metrics");
    assert_eq!(metrics.viewport_height(), 40.0);
    assert!(
        metrics.content_height() > 40.0,
        "wrapped content should exceed the viewport, got {}",
        metrics.content_height()
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
fn invalid_text_measurements_keep_an_empty_semantic_record() {
    struct InvalidMeasurer;
    impl TextMeasurer for InvalidMeasurer {
        fn measure(&self, _: &str, _: u16) -> UiSize {
            UiSize::new(f32::NAN, -1.0)
        }
    }

    let document = UiDocument::new(text("title", "Visible")).expect("document is valid");
    let layout = document.layout(UiRect::from_size(0.0, 0.0, 100.0, 100.0), &InvalidMeasurer);
    assert_eq!(layout.items().len(), 1);
    assert!(layout.items()[0].bounds().is_empty());
    assert!(layout.bounds(&id("title")).is_none());
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
