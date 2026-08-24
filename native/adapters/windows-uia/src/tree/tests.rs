//! Focused verification for immutable UI Automation tree construction.

use anodrel_ui::{ElementId, UiEvent, UiRect};
use anodrel_ui_session::{SessionInteractionCandidate, UiDocumentSession, UiInputMailbox};
use anodrel_windows_accessibility::{AccessibleElement, ClientOrigin, accessible_elements};

use super::{ROOT_AUTOMATION_ID, Tree, direction};
use crate::{UiAutomationActionSink, UiAutomationFocusMailbox, raw::VT_EMPTY};

const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"go","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"},{"id":"blocked","kind":"action","label":"Unavailable","fontSize":16,"enabled":false,"tone":"accent"}]}}"#;
const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"name","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true},{"id":"locked","kind":"field","label":"Locked","value":"","maxLength":64,"fontSize":16,"enabled":false},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;
const STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"result","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}"#;
const NESTED_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"section","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"detail","kind":"text","value":"Nested","fontSize":16,"tone":"primary"},{"id":"go","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]},{"id":"footer","kind":"text","value":"Done","fontSize":16,"tone":"primary"}]}}"#;

struct FixedMeasurer;

impl anodrel_ui::TextMeasurer for FixedMeasurer {
    fn measure(&self, text: &str, font_size: u16) -> anodrel_ui::UiSize {
        anodrel_ui::UiSize::new(
            text.chars().count() as f32 * f32::from(font_size) * 0.5,
            f32::from(font_size),
        )
    }
}

/// Maps the fixture document exactly as the host would.
fn mapped() -> Vec<AccessibleElement> {
    mapped_document(DOCUMENT)
}

fn mapped_document(source: &str) -> Vec<AccessibleElement> {
    let document = anodrel_ui_document::decode(source).expect("the fixture document is valid");
    map_document(document)
}

fn mapped_document_v3(source: &str) -> Vec<AccessibleElement> {
    let document =
        anodrel_ui_document::decode_v3(source).expect("the v3 fixture document is valid");
    map_document(document)
}

fn map_document(document: anodrel_ui::UiDocument) -> Vec<AccessibleElement> {
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
    accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    )
}

fn field_mapped() -> Vec<AccessibleElement> {
    mapped_document(FIELD_DOCUMENT)
}

fn tree() -> Tree {
    Tree::new(
        "Window".encode_utf16().collect(),
        mapped(),
        Vec::new(),
        None,
        None,
        None,
    )
}

fn tree_with_action_sink() -> (Tree, UiInputMailbox, anodrel_ui_session::UiDocumentRevision) {
    let mut session = UiDocumentSession::new();
    let revision = session
        .replace_document(DOCUMENT)
        .expect("the fixture document is valid");
    let mailbox = UiInputMailbox::new();
    let sink = UiAutomationActionSink::for_current_session(revision, mailbox.clone())
        .expect("an accepted document has an action route");
    (
        Tree::new(
            "Window".encode_utf16().collect(),
            mapped(),
            Vec::new(),
            None,
            Some(sink),
            None,
        ),
        mailbox,
        revision,
    )
}

fn focus_sink() -> crate::UiAutomationFocusSink {
    let mailbox = UiAutomationFocusMailbox::new();
    let route = mailbox.route(None);
    let completing = mailbox.clone();
    route.with_notifier(move || {
        let request = completing.take().expect("a focus route is pending");
        completing.complete(request.id(), true)
    })
}

#[test]
fn a_visible_group_is_published_with_its_children() {
    let tree = tree();
    assert_eq!(tree.step(None, direction::FIRST_CHILD), Some(Some(0)));
    assert_eq!(tree.step(Some(0), direction::FIRST_CHILD), Some(Some(1)));
    assert_eq!(tree.step(Some(0), direction::LAST_CHILD), Some(Some(3)));
}

#[test]
fn the_root_walks_to_its_first_and_last_child() {
    let tree = tree();
    assert_eq!(tree.step(None, direction::FIRST_CHILD), Some(Some(0)));
    assert_eq!(tree.step(None, direction::LAST_CHILD), Some(Some(0)));
    // A window with no semantic elements has no children at all.
    let empty = Tree::new(Vec::new(), Vec::new(), Vec::new(), None, None, None);
    assert_eq!(empty.step(None, direction::FIRST_CHILD), None);
    // The root's parent belongs to Windows, not to this provider.
    assert_eq!(tree.step(None, direction::PARENT), None);
}

#[test]
fn nested_elements_walk_only_their_direct_relationships() {
    let tree = Tree::new(
        Vec::new(),
        mapped_document(NESTED_DOCUMENT),
        Vec::new(),
        None,
        None,
        None,
    );

    // root → [heading, section → [detail, go], footer]
    assert_eq!(tree.step(None, direction::FIRST_CHILD), Some(Some(0)));
    assert_eq!(tree.step(Some(0), direction::PARENT), Some(None));
    assert_eq!(tree.step(Some(0), direction::FIRST_CHILD), Some(Some(1)));
    assert_eq!(tree.step(Some(0), direction::LAST_CHILD), Some(Some(5)));
    assert_eq!(tree.step(Some(1), direction::NEXT_SIBLING), Some(Some(2)));
    assert_eq!(
        tree.step(Some(2), direction::PREVIOUS_SIBLING),
        Some(Some(1))
    );
    assert_eq!(tree.step(Some(2), direction::NEXT_SIBLING), Some(Some(5)));
    assert_eq!(tree.step(Some(2), direction::PARENT), Some(Some(0)));
    assert_eq!(tree.step(Some(2), direction::FIRST_CHILD), Some(Some(3)));
    assert_eq!(tree.step(Some(2), direction::LAST_CHILD), Some(Some(4)));
    assert_eq!(tree.step(Some(3), direction::PARENT), Some(Some(2)));
    assert_eq!(tree.step(Some(3), direction::NEXT_SIBLING), Some(Some(4)));
    assert_eq!(
        tree.step(Some(4), direction::PREVIOUS_SIBLING),
        Some(Some(3))
    );
    assert_eq!(tree.step(Some(5), direction::NEXT_SIBLING), None);
}

#[test]
fn the_root_reports_its_own_fixed_identity() {
    let tree = tree();
    assert!(tree.property(None, super::property::NAME).is_some());
    assert!(
        tree.property(None, super::property::AUTOMATION_ID)
            .is_some()
    );
    // An unsupported property is not supplied rather than guessed at.
    assert!(tree.property(None, 30_006).is_none());
    assert_eq!(ROOT_AUTOMATION_ID, "anodrel.surface");
}

#[test]
fn an_element_reports_its_mapped_values_and_nothing_more() {
    let tree = tree();
    let control_type = tree
        .property(Some(1), super::property::CONTROL_TYPE)
        .expect("a mapped element supplies its control type");
    assert_ne!(control_type.vt, VT_EMPTY);
    assert!(tree.property(Some(1), 30_006).is_none());
    // An index past the end is not a panic and not a guess.
    assert!(tree.property(Some(99), super::property::NAME).is_none());
}

#[test]
fn a_status_publishes_its_declared_live_setting() {
    let tree = Tree::new(
        Vec::new(),
        mapped_document_v3(STATUS_DOCUMENT),
        Vec::new(),
        None,
        None,
        None,
    );

    let live_setting = tree
        .property(Some(0), super::property::LIVE_SETTING)
        .expect("a status supplies its live setting");
    assert_eq!(live_setting.int_value(), Some(super::live_setting::POLITE));
}

#[test]
fn only_elements_carry_a_runtime_identifier_and_bounds() {
    let tree = tree();
    assert!(tree.runtime_id(None).is_none());
    assert!(tree.bounds(None).is_none());
    assert!(tree.runtime_id(Some(0)).is_some());
    assert!(tree.bounds(Some(0)).is_some());
    assert!(tree.runtime_id(Some(99)).is_none());
}

#[test]
fn hit_testing_finds_the_deepest_non_container_element_at_its_own_bounds() {
    // Asking with each element's own rectangle keeps the test about hit
    // testing rather than about whatever geometry the layout produced.
    let published = mapped();
    let tree = Tree::new(Vec::new(), published.clone(), Vec::new(), None, None, None);

    for (index, element) in published.iter().enumerate().skip(1) {
        let bounds = element.bounds();
        let x = bounds.left + bounds.width / 2.0;
        let y = bounds.top + bounds.height / 2.0;
        assert_eq!(
            tree.element_at(x, y),
            Some(index),
            "{} was not found inside its own bounds",
            element.automation_id()
        );
        // The right and bottom edges are exclusive, so the neighbouring
        // pixel must not report this element.
        assert_ne!(
            tree.element_at(bounds.left + bounds.width, y),
            Some(index),
            "{} claimed its exclusive right edge",
            element.automation_id()
        );
    }
}

#[test]
fn a_point_outside_every_element_reports_nothing() {
    assert_eq!(tree().element_at(-50.0, -50.0), None);
    assert_eq!(tree().element_at(100_000.0, 100_000.0), None);
}

#[test]
fn focus_reports_only_a_visible_enabled_published_target() {
    let published = mapped();
    let focused = Tree::new(
        Vec::new(),
        published.clone(),
        Vec::new(),
        Some(ElementId::new("go").expect("fixed ID is valid")),
        None,
        None,
    );
    assert_eq!(focused.focused(), Some(2));
    assert_eq!(
        focused
            .property(Some(2), crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
            .expect("the focus property is supplied")
            .boolean_value(),
        Some(true)
    );
    assert_eq!(
        focused
            .property(Some(1), crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
            .expect("the focus property is supplied")
            .boolean_value(),
        Some(false)
    );
    assert_eq!(
        focused
            .property(None, crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
            .expect("the root focus property is supplied")
            .boolean_value(),
        Some(false)
    );

    for id in ["heading", "blocked", "missing"] {
        let tree = Tree::new(
            Vec::new(),
            published.clone(),
            Vec::new(),
            Some(ElementId::new(id).expect("fixed ID is valid")),
            None,
            None,
        );
        assert_eq!(tree.focused(), None, "{id} must not report focus");
    }
}

#[test]
fn focus_route_accepts_only_a_visible_enabled_focus_target() {
    let tree = Tree::new(
        Vec::new(),
        mapped(),
        Vec::new(),
        None,
        None,
        Some(focus_sink()),
    );

    // The root group and text remain non-focusable. Only the enabled action
    // can enter the route, and success updates this provider's own snapshot.
    assert!(!tree.supports_focus(0));
    assert!(!tree.supports_focus(1));
    assert!(tree.supports_focus(2));
    assert!(!tree.supports_focus(3));
    assert!(tree.focus(2));
    assert_eq!(tree.focused(), Some(2));
    assert!(!tree.focus(3));
    assert_eq!(tree.focused(), Some(2));
}

#[test]
fn field_values_are_visible_only_on_matching_edit_elements() {
    let tree = Tree::new(
        Vec::new(),
        field_mapped(),
        vec![
            (
                ElementId::new("name").expect("fixed ID is valid"),
                "Ada".to_owned(),
            ),
            (
                ElementId::new("locked").expect("fixed ID is valid"),
                "Still readable".to_owned(),
            ),
            (
                ElementId::new("continue").expect("fixed ID is valid"),
                "must not reach an action".to_owned(),
            ),
        ],
        None,
        None,
        None,
    );

    assert!(tree.supports_value(1));
    assert!(
        tree.supports_value(2),
        "a disabled visible field is readable"
    );
    assert!(!tree.supports_value(3), "an action never exposes a value");
    assert_eq!(
        String::from_utf16(tree.field_value(1).expect("the name value exists"))
            .expect("the value is valid UTF-16"),
        "Ada"
    );
    let property = tree
        .property(Some(1), crate::raw5::UIA_VALUE_VALUE_PROPERTY_ID)
        .expect("the value property is supplied");
    // SAFETY: this test owns the BSTR allocated for its Variant result.
    assert_eq!(
        unsafe { property.copy_and_free_string() },
        Some("Ada".to_owned())
    );
    assert_eq!(
        tree.property(Some(1), crate::raw5::UIA_VALUE_IS_READ_ONLY_PROPERTY_ID)
            .expect("the read-only property is supplied")
            .boolean_value(),
        Some(true)
    );
    assert!(
        tree.property(Some(3), crate::raw5::UIA_VALUE_VALUE_PROPERTY_ID)
            .is_none(),
        "a non-field never receives a value property"
    );
}

#[test]
fn invoke_is_limited_to_an_enabled_authenticated_session_button() {
    let read_only = tree();
    // Text cannot invoke, and a product-looking button without a session
    // action route remains readable but non-invokable.
    assert!(!read_only.supports_invoke(0));
    assert!(!read_only.supports_invoke(1));
    assert!(!read_only.supports_invoke(2));

    let (tree, mailbox, revision) = tree_with_action_sink();
    assert!(!tree.supports_invoke(0), "groups must not expose Invoke");
    assert!(!tree.supports_invoke(1), "text must not expose Invoke");
    assert!(tree.supports_invoke(2), "the enabled action is invokable");
    assert!(
        !tree.supports_invoke(3),
        "a disabled button must not expose Invoke"
    );
    assert!(!tree.invoke(0));
    assert!(!tree.invoke(3));
    assert!(tree.invoke(2));

    let batch = mailbox.drain();
    assert_eq!(batch.dropped(), 0);
    let candidates = batch.into_candidates();
    assert_eq!(candidates.len(), 1);
    let SessionInteractionCandidate::Ui(candidate) =
        candidates.into_iter().next().expect("one candidate")
    else {
        panic!("the invocation must produce a document candidate");
    };
    let (candidate_revision, event) = candidate.into_parts();
    assert_eq!(candidate_revision, revision);
    assert_eq!(
        event,
        UiEvent::ActionInvoked(ElementId::new("go").expect("fixed ID is valid"))
    );
}
