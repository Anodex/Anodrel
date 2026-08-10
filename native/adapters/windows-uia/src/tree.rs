//! The element tree one provider publishes, and the pure logic over it.
//!
//! Keeping navigation, property lookup, and hit testing here means the COM
//! layer holds only pointers and reference counts, and every rule about what a
//! client can see is testable without Windows.

use anodrel_windows_accessibility::{AccessibleElement, ScreenRect, property};

use crate::raw::{CONTROL_TYPE_WINDOW, Variant};
use crate::raw2::{UiaRect, direction};

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
}

impl Tree {
    /// Builds the tree for one window title and its publishable elements.
    #[must_use]
    pub fn new(title: Vec<u16>, elements: Vec<AccessibleElement>) -> Self {
        Self { title, elements }
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
    use anodrel_ui::UiRect;
    use anodrel_windows_accessibility::{
        AccessibleElement, ClientOrigin, accessible_elements, control_type,
    };

    use super::{ROOT_AUTOMATION_ID, Tree, direction, publishable, step};
    use crate::raw::VT_EMPTY;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"go","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

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

    fn tree() -> Tree {
        Tree::new("Window".encode_utf16().collect(), publishable(mapped()))
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
        let tree = Tree::new(Vec::new(), published.clone());

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
}
