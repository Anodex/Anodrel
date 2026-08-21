//! The element tree one provider publishes, and the pure logic over it.
//!
//! Keeping navigation, property lookup, and hit testing here means the COM
//! layer holds only pointers and reference counts, and every rule about what a
//! client can see is testable without Windows.

use std::{collections::BTreeMap, sync::Mutex};

use anodrel_ui::ElementId;
use anodrel_windows_accessibility::{AccessibleElement, ScreenRect, control_type, property};

use crate::raw::{CONTROL_TYPE_WINDOW, Variant};
use crate::raw2::{UiaRect, direction};
use crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID;
use crate::raw5::{UIA_VALUE_IS_READ_ONLY_PROPERTY_ID, UIA_VALUE_VALUE_PROPERTY_ID};
use crate::{UiAutomationActionSink, UiAutomationFocusSink};

/// The fixed automation identifier for an Anodrel surface's root.
///
/// Host-owned text. An application cannot supply or change it.
pub const ROOT_AUTOMATION_ID: &str = "anodrel.surface";

/// Selects the elements worth publishing to assistive technology.
///
/// The published tree is flat, so a container carries no meaning: a group whose
/// children sit beside it rather than inside it would be announced as an empty
/// thing to step through. Only elements that say or do something are kept, and
/// grouping waits for a hierarchical tree.
#[must_use]
pub fn publishable(elements: Vec<AccessibleElement>) -> Vec<AccessibleElement> {
    elements
        .into_iter()
        .filter(|element| {
            element.control_type() != anodrel_windows_accessibility::control_type::GROUP
        })
        .collect()
}

/// One window's published accessibility tree.
#[derive(Debug)]
pub struct Tree {
    title: Vec<u16>,
    elements: Vec<AccessibleElement>,
    field_values: BTreeMap<String, Vec<u16>>,
    /// A provider's initial snapshot, updated only after that same provider's
    /// successful `SetFocus` call. It never observes unrelated live focus.
    focused: Mutex<Option<usize>>,
    action_sink: Option<UiAutomationActionSink>,
    focus_sink: Option<UiAutomationFocusSink>,
}

impl Tree {
    /// Builds the tree for one window title and its publishable elements.
    ///
    /// Focus and field values are reduced to published targets while the tree
    /// is created, so a provider never reads a mutable view. The action sink
    /// exists only for a current authenticated UI session. The focus sink is a
    /// host-only route for this one view; both are gated per element below.
    #[must_use]
    pub fn new(
        title: Vec<u16>,
        elements: Vec<AccessibleElement>,
        field_values: Vec<(ElementId, String)>,
        focused: Option<ElementId>,
        action_sink: Option<UiAutomationActionSink>,
        focus_sink: Option<UiAutomationFocusSink>,
    ) -> Self {
        let focused = focused.and_then(|id| focus_index(&elements, &id));
        let field_values = field_values
            .into_iter()
            .map(|(id, value)| (id.as_str().to_owned(), utf16(&value)))
            .collect();
        Self {
            title,
            elements,
            field_values,
            focused: Mutex::new(focused),
            action_sink,
            focus_sink,
        }
    }

    /// Returns how many elements the window publishes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns the value for one UI Automation property, if this provider
    /// supplies it.
    ///
    /// `None` means "not supplied", which the caller reports as an empty
    /// variant rather than a failure.
    #[must_use]
    pub fn property(&self, element: Option<usize>, requested: i32) -> Option<Variant> {
        match element {
            None => self.root_property(requested),
            Some(index) => self.element_property(index, requested),
        }
    }

    fn root_property(&self, requested: i32) -> Option<Variant> {
        match requested {
            property::NAME => Some(Variant::string(&self.title)),
            property::CONTROL_TYPE => Some(Variant::int(CONTROL_TYPE_WINDOW)),
            property::AUTOMATION_ID => Some(Variant::string(&utf16(ROOT_AUTOMATION_ID))),
            property::IS_CONTROL_ELEMENT | property::IS_CONTENT_ELEMENT => {
                Some(Variant::boolean(true))
            }
            property::IS_ENABLED => Some(Variant::boolean(true)),
            // The window is a container, not a target.
            property::IS_KEYBOARD_FOCUSABLE => Some(Variant::boolean(false)),
            UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID => Some(Variant::boolean(false)),
            _ => None,
        }
    }

    fn element_property(&self, index: usize, requested: i32) -> Option<Variant> {
        let element = self.elements.get(index)?;
        match requested {
            property::NAME => Some(Variant::string(&utf16(element.name()))),
            property::CONTROL_TYPE => Some(Variant::int(element.control_type())),
            property::AUTOMATION_ID => Some(Variant::string(&utf16(element.automation_id()))),
            property::IS_ENABLED => Some(Variant::boolean(element.enabled())),
            property::IS_KEYBOARD_FOCUSABLE => Some(Variant::boolean(element.keyboard_focusable())),
            UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID => {
                Some(Variant::boolean(self.focused() == Some(index)))
            }
            UIA_VALUE_VALUE_PROPERTY_ID => Some(Variant::string(self.field_value(index)?)),
            UIA_VALUE_IS_READ_ONLY_PROPERTY_ID => {
                self.field_value(index).map(|_| Variant::boolean(true))
            }
            property::IS_CONTROL_ELEMENT | property::IS_CONTENT_ELEMENT => {
                Some(Variant::boolean(true))
            }
            _ => None,
        }
    }

    /// Returns one element's runtime identifier.
    ///
    /// The window root has none of its own: Windows supplies one through the
    /// host provider.
    #[must_use]
    pub fn runtime_id(&self, element: Option<usize>) -> Option<[i32; 2]> {
        Some(self.elements.get(element?)?.runtime_id())
    }

    /// Returns one element's bounding rectangle, or `None` for the window root,
    /// whose rectangle the host provider already supplies.
    #[must_use]
    pub fn bounds(&self, element: Option<usize>) -> Option<UiaRect> {
        let bounds = self.elements.get(element?)?.bounds();
        Some(to_uia_rect(bounds))
    }

    /// Returns the topmost element containing a screen point.
    ///
    /// Later elements win, matching the painter's order the surface draws in,
    /// so the thing visually on top is the thing reported.
    #[must_use]
    pub fn element_at(&self, x: f64, y: f64) -> Option<usize> {
        self.elements
            .iter()
            .enumerate()
            .rev()
            .find(|(_, element)| contains(element.bounds(), x, y))
            .map(|(index, _)| index)
    }

    /// Returns the position of this immutable tree's focused child, if any.
    ///
    /// A missing, clipped, disabled, non-focusable, or filtered ID is reduced
    /// to no focus at construction time. See Decision 0070.
    #[must_use]
    pub fn focused(&self) -> Option<usize> {
        *self
            .focused
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether one published element exposes a read-only field-value snapshot.
    ///
    /// A value reaches only a matching visible Edit element. No other role can
    /// become a value control just because it shares an element ID with a host
    /// field state. See Decision 0071.
    #[must_use]
    pub fn supports_value(&self, index: usize) -> bool {
        self.field_value(index).is_some()
    }

    /// Returns one visible field's immutable UTF-16 value snapshot.
    ///
    /// Only the Value COM binding uses this. The text has already been copied
    /// from host-owned state and does not provide a route back to an
    /// application. See Decision 0071.
    #[must_use]
    pub fn value(&self, index: usize) -> Option<&[u16]> {
        self.field_value(index)
    }

    /// Whether one published element supports the bounded Invoke pattern.
    ///
    /// The control type, enabled state, current authenticated-session sink, and
    /// semantic ID are all required. A malformed value is not an action merely
    /// because it was labelled a button in an externally constructed tree.
    #[must_use]
    pub fn supports_invoke(&self, index: usize) -> bool {
        self.invocation_id(index).is_some() && self.action_sink.is_some()
    }

    /// Offers exactly one revision-bound semantic button action to the session.
    ///
    /// `false` covers every refusal: no current session, a role that does not
    /// invoke, a disabled button, an invalid ID, or a full bounded mailbox.
    /// None of those paths can perform a native action or call an application.
    pub fn invoke(&self, index: usize) -> bool {
        let Some(id) = self.invocation_id(index) else {
            return false;
        };
        let Some(sink) = &self.action_sink else {
            return false;
        };
        sink.offer(id)
    }

    /// Whether one published element supports host-owned UI Automation focus.
    ///
    /// This is separate from button invocation: a field can take focus but
    /// cannot create a semantic action, and a diagnostic view can take focus
    /// without gaining an application action route.
    #[must_use]
    pub fn supports_focus(&self, index: usize) -> bool {
        self.focus_target(index).is_some() && self.focus_sink.is_some()
    }

    /// Requests focus for one published element through its host-only route.
    ///
    /// The owner still validates the current layout and snapshot revision
    /// before it writes focus. On success only this tree's copied focus result
    /// changes, so an immediate query stays truthful without a live lookup.
    pub fn focus(&self, index: usize) -> bool {
        let Some(target) = self.focus_target(index) else {
            return false;
        };
        let Some(sink) = &self.focus_sink else {
            return false;
        };
        if !sink.focus(target) {
            return false;
        }
        *self
            .focused
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(index);
        true
    }

    fn invocation_id(&self, index: usize) -> Option<ElementId> {
        let element = self.elements.get(index)?;
        (element.control_type() == control_type::BUTTON && element.enabled())
            .then(|| ElementId::new(element.automation_id()).ok())
            .flatten()
    }

    fn focus_target(&self, index: usize) -> Option<ElementId> {
        let element = self.elements.get(index)?;
        let id = ElementId::new(element.automation_id()).ok()?;
        (focus_index(&self.elements, &id) == Some(index)).then_some(id)
    }

    fn field_value(&self, index: usize) -> Option<&[u16]> {
        let element = self.elements.get(index)?;
        (element.control_type() == control_type::EDIT)
            .then(|| {
                self.field_values
                    .get(element.automation_id())
                    .map(Vec::as_slice)
            })
            .flatten()
    }
}

fn focus_index(elements: &[AccessibleElement], id: &ElementId) -> Option<usize> {
    elements.iter().position(|element| {
        element.automation_id() == id.as_str()
            && element.enabled()
            && element.keyboard_focusable()
            && element.bounds().width > 0.0
            && element.bounds().height > 0.0
    })
}

/// Resolves one navigation step over a flat tree of `count` elements.
///
/// `None` as the current element means the window root. A `None` result means
/// there is nothing in that direction, which UI Automation expects as a null
/// rather than a failure.
#[must_use]
pub fn step(element: Option<usize>, towards: i32, count: usize) -> Option<Option<usize>> {
    match (element, towards) {
        // The root's parent belongs to Windows, not to this provider.
        (None, direction::PARENT) => None,
        (None, direction::FIRST_CHILD) => (count > 0).then_some(Some(0)),
        (None, direction::LAST_CHILD) => count.checked_sub(1).map(Some),
        (None, _) => None,

        (Some(_), direction::PARENT) => Some(None),
        // A flat tree has no grandchildren.
        (Some(_), direction::FIRST_CHILD | direction::LAST_CHILD) => None,
        (Some(index), direction::NEXT_SIBLING) => {
            let next = index.checked_add(1)?;
            (next < count).then_some(Some(next))
        }
        (Some(index), direction::PREVIOUS_SIBLING) => index.checked_sub(1).map(Some),
        (Some(_), _) => None,
    }
}

fn contains(bounds: ScreenRect, x: f64, y: f64) -> bool {
    bounds.width > 0.0
        && bounds.height > 0.0
        && x >= bounds.left
        && y >= bounds.top
        && x < bounds.left + bounds.width
        && y < bounds.top + bounds.height
}

const fn to_uia_rect(bounds: ScreenRect) -> UiaRect {
    UiaRect {
        left: bounds.left,
        top: bounds.top,
        width: bounds.width,
        height: bounds.height,
    }
}

fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use anodrel_ui::{ElementId, UiEvent, UiRect};
    use anodrel_ui_session::{UiDocumentSession, UiInputMailbox};
    use anodrel_windows_accessibility::{
        AccessibleElement, ClientOrigin, accessible_elements, control_type,
    };

    use super::{ROOT_AUTOMATION_ID, Tree, direction, publishable, step};
    use crate::{UiAutomationActionSink, UiAutomationFocusMailbox, raw::VT_EMPTY};

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"go","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"},{"id":"blocked","kind":"action","label":"Unavailable","fontSize":16,"enabled":false,"tone":"accent"}]}}"#;
    const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"name","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true},{"id":"locked","kind":"field","label":"Locked","value":"","maxLength":64,"fontSize":16,"enabled":false},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

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
        let document =
            anodrel_ui_document::decode(DOCUMENT).expect("the fixture document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        )
    }

    fn field_mapped() -> Vec<AccessibleElement> {
        let document =
            anodrel_ui_document::decode(FIELD_DOCUMENT).expect("the field fixture is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        )
    }

    fn tree() -> Tree {
        Tree::new(
            "Window".encode_utf16().collect(),
            publishable(mapped()),
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
                publishable(mapped()),
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
    fn a_group_is_not_published_to_a_flat_tree() {
        // A container whose children sit beside it rather than inside it would
        // be announced as an empty thing to step through.
        let all = mapped();
        assert!(
            all.iter()
                .any(|element| element.control_type() == control_type::GROUP),
            "the fixture must contain a container to filter"
        );

        let published = publishable(all);
        assert!(!published.is_empty());
        assert!(
            published
                .iter()
                .all(|element| element.control_type() != control_type::GROUP)
        );
    }

    #[test]
    fn the_root_walks_to_its_first_and_last_child() {
        assert_eq!(step(None, direction::FIRST_CHILD, 2), Some(Some(0)));
        assert_eq!(step(None, direction::LAST_CHILD, 2), Some(Some(1)));
        // A window with nothing to publish has no children at all.
        assert_eq!(step(None, direction::FIRST_CHILD, 0), None);
        // The root's parent belongs to Windows, not to this provider.
        assert_eq!(step(None, direction::PARENT, 2), None);
    }

    #[test]
    fn elements_walk_to_their_siblings_and_stop_at_the_ends() {
        assert_eq!(step(Some(0), direction::NEXT_SIBLING, 2), Some(Some(1)));
        assert_eq!(step(Some(1), direction::NEXT_SIBLING, 2), None);
        assert_eq!(step(Some(1), direction::PREVIOUS_SIBLING, 2), Some(Some(0)));
        assert_eq!(step(Some(0), direction::PREVIOUS_SIBLING, 2), None);
        assert_eq!(step(Some(0), direction::PARENT, 2), Some(None));
        // A flat tree has no grandchildren.
        assert_eq!(step(Some(0), direction::FIRST_CHILD, 2), None);
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
    fn only_elements_carry_a_runtime_identifier_and_bounds() {
        let tree = tree();
        assert!(tree.runtime_id(None).is_none());
        assert!(tree.bounds(None).is_none());
        assert!(tree.runtime_id(Some(0)).is_some());
        assert!(tree.bounds(Some(0)).is_some());
        assert!(tree.runtime_id(Some(99)).is_none());
    }

    #[test]
    fn hit_testing_finds_each_element_within_its_own_reported_bounds() {
        // Asking with each element's own rectangle keeps the test about hit
        // testing rather than about whatever geometry the layout produced.
        let published = publishable(mapped());
        let tree = Tree::new(Vec::new(), published.clone(), Vec::new(), None, None, None);

        for (index, element) in published.iter().enumerate() {
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
        let published = publishable(mapped());
        let focused = Tree::new(
            Vec::new(),
            published.clone(),
            Vec::new(),
            Some(ElementId::new("go").expect("fixed ID is valid")),
            None,
            None,
        );
        // The stack was filtered, leaving text, the enabled action, then the
        // disabled action. The portable focus target therefore maps directly
        // to the enabled button's published position.
        assert_eq!(focused.focused(), Some(1));
        assert_eq!(
            focused
                .property(Some(1), crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
                .expect("the focus property is supplied")
                .boolean_value(),
            Some(true)
        );
        assert_eq!(
            focused
                .property(Some(0), crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
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
            publishable(mapped()),
            Vec::new(),
            None,
            None,
            Some(focus_sink()),
        );

        // The stack was filtered, then the text, the enabled action, and the
        // disabled action remained. Only the enabled action can enter the
        // route, and success updates this provider's own focus snapshot.
        assert!(!tree.supports_focus(0));
        assert!(tree.supports_focus(1));
        assert!(!tree.supports_focus(2));
        assert!(tree.focus(1));
        assert_eq!(tree.focused(), Some(1));
        assert!(!tree.focus(2));
        assert_eq!(tree.focused(), Some(1));
    }

    #[test]
    fn field_values_are_visible_only_on_matching_edit_elements() {
        let tree = Tree::new(
            Vec::new(),
            publishable(field_mapped()),
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

        assert!(tree.supports_value(0));
        assert!(
            tree.supports_value(1),
            "a disabled visible field is readable"
        );
        assert!(!tree.supports_value(2), "an action never exposes a value");
        assert_eq!(
            String::from_utf16(tree.field_value(0).expect("the name value exists"))
                .expect("the value is valid UTF-16"),
            "Ada"
        );
        let property = tree
            .property(Some(0), crate::raw5::UIA_VALUE_VALUE_PROPERTY_ID)
            .expect("the value property is supplied");
        // SAFETY: this test owns the BSTR allocated for its Variant result.
        assert_eq!(
            unsafe { property.copy_and_free_string() },
            Some("Ada".to_owned())
        );
        assert_eq!(
            tree.property(Some(0), crate::raw5::UIA_VALUE_IS_READ_ONLY_PROPERTY_ID)
                .expect("the read-only property is supplied")
                .boolean_value(),
            Some(true)
        );
        assert!(
            tree.property(Some(2), crate::raw5::UIA_VALUE_VALUE_PROPERTY_ID)
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

        let (tree, mailbox, revision) = tree_with_action_sink();
        assert!(!tree.supports_invoke(0), "text must not expose Invoke");
        assert!(tree.supports_invoke(1), "the enabled action is invokable");
        assert!(
            !tree.supports_invoke(2),
            "a disabled button must not expose Invoke"
        );
        assert!(!tree.invoke(0));
        assert!(!tree.invoke(2));
        assert!(tree.invoke(1));

        let batch = mailbox.drain();
        assert_eq!(batch.dropped(), 0);
        let candidates = batch.into_candidates();
        assert_eq!(candidates.len(), 1);
        let (candidate_revision, event) = candidates
            .into_iter()
            .next()
            .expect("one candidate")
            .into_parts();
        assert_eq!(candidate_revision, revision);
        assert_eq!(
            event,
            UiEvent::ActionInvoked(ElementId::new("go").expect("fixed ID is valid"))
        );
    }
}
