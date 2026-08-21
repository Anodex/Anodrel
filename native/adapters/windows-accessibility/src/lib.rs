#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! The Windows accessibility mapping for Anodrel's owned semantic snapshot.
//!
//! This adapter is a pure function from one [`UiAccessibilitySnapshot`] to the
//! values Microsoft UI Automation asks for. It performs no operating-system
//! call, holds no lock, retains no native object, and cannot fail.
//!
//! The boundary runs one way. Nothing here reads the accessibility tree back,
//! reports focus, delivers an announcement, or reveals that an assistive
//! technology is present — an application supplies a UI document and learns
//! nothing in return. See `docs/ACCESSIBILITY.md` and Decision 0063.
//!
//! `anodrel-windows-uia` publishes this mapping to Windows. It keeps the
//! mapping pure; its separately bounded Invoke implementation is defined by
//! Decision 0069.

mod geometry;
mod uia;

use anodrel_ui::{UiAccessibilityNode, UiAccessibilityRole, UiAccessibilitySnapshot};

pub use geometry::{ClientOrigin, ScreenRect, screen_rect};
pub use uia::{UIA_APPEND_RUNTIME_ID, control_type, property};

/// One snapshot node expressed in UI Automation terms.
///
/// Every field is derived from semantics the application already declared. None
/// of them is application-supplied accessibility data, because the document
/// format has no accessibility field to supply.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessibleElement {
    automation_id: String,
    name: String,
    control_type: i32,
    enabled: bool,
    keyboard_focusable: bool,
    bounds: ScreenRect,
    runtime_id: [i32; 2],
}

impl AccessibleElement {
    /// The document element ID, reported as `UIA_AutomationIdPropertyId`.
    ///
    /// This is a semantic identifier already present in the document and
    /// bounded to 64 ASCII characters. It is not a path, handle, or secret.
    #[must_use]
    pub fn automation_id(&self) -> &str {
        &self.automation_id
    }

    /// The accessible name, empty where the role has none.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The `UIA_ControlTypePropertyId` value.
    #[must_use]
    pub const fn control_type(&self) -> i32 {
        self.control_type
    }

    /// Whether the element is enabled for a person's interaction.
    ///
    /// Only buttons and fields can be disabled.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether keyboard focus can reach this element.
    #[must_use]
    pub const fn keyboard_focusable(&self) -> bool {
        self.keyboard_focusable
    }

    /// The `UIA_BoundingRectanglePropertyId` value in physical screen pixels.
    #[must_use]
    pub const fn bounds(&self) -> ScreenRect {
        self.bounds
    }

    /// The runtime ID Windows appends to the host window's own.
    ///
    /// The identifier is positional, so it is stable for as long as the tree
    /// is. Replacing the UI document produces a new tree whose runtime IDs are
    /// expected to differ.
    #[must_use]
    pub const fn runtime_id(&self) -> [i32; 2] {
        self.runtime_id
    }
}

/// Maps one visible snapshot into UI Automation elements, in source order.
///
/// The result is plain data. Handing it to Windows is the provider's job and is
/// deliberately not part of this adapter.
#[must_use]
pub fn accessible_elements(
    snapshot: &UiAccessibilitySnapshot,
    origin: ClientOrigin,
) -> Vec<AccessibleElement> {
    snapshot
        .nodes()
        .iter()
        .enumerate()
        .map(|(index, node)| accessible_element(node, index, origin))
        .collect()
}

/// Maps one snapshot node at its source-order position.
#[must_use]
pub fn accessible_element(
    node: &UiAccessibilityNode,
    index: usize,
    origin: ClientOrigin,
) -> AccessibleElement {
    AccessibleElement {
        automation_id: node.id().as_str().to_owned(),
        // A role with no name reports an empty string rather than nothing:
        // UI Automation has no absent-name representation, and an empty name is
        // what "this element is not named" means there.
        name: node.name().unwrap_or_default().to_owned(),
        control_type: control_type_for(node.role()),
        enabled: is_enabled(node),
        keyboard_focusable: keyboard_focusable(node.role()),
        bounds: screen_rect(node.bounds(), origin),
        runtime_id: [UIA_APPEND_RUNTIME_ID, runtime_index(index)],
    }
}

/// Returns the UI Automation control type for one portable role.
#[must_use]
pub const fn control_type_for(role: UiAccessibilityRole) -> i32 {
    match role {
        UiAccessibilityRole::Group => control_type::GROUP,
        UiAccessibilityRole::StaticText => control_type::TEXT,
        UiAccessibilityRole::Button => control_type::BUTTON,
        UiAccessibilityRole::Edit => control_type::EDIT,
    }
}

/// Returns the `IsEnabled` value for one node.
///
/// UI Automation reads `IsEnabled` as "can be interacted with", and a screen
/// reader announces a disabled element as unavailable. Only an action or a
/// field can be unavailable; text and containers are not interactive in the
/// first place, so reporting the snapshot's flag for them would have Narrator
/// describe ordinary prose as dimmed and out of reach.
#[must_use]
pub fn is_enabled(node: &UiAccessibilityNode) -> bool {
    match node.role() {
        UiAccessibilityRole::Button | UiAccessibilityRole::Edit => node.enabled(),
        UiAccessibilityRole::Group | UiAccessibilityRole::StaticText => true,
    }
}

/// Returns whether keyboard focus can reach one portable role.
///
/// This matches the portable focus traversal exactly: an action and a field
/// take focus, so assistive technology and the keyboard agree on what is
/// reachable. Reporting a field as unfocusable would be a plain lie to a screen
/// reader — the pure mapping's one-directional rule is about publishing
/// semantics, not about misdescribing the surface. UI Automation invocation is
/// separately bounded by Decision 0069.
#[must_use]
pub const fn keyboard_focusable(role: UiAccessibilityRole) -> bool {
    matches!(
        role,
        UiAccessibilityRole::Button | UiAccessibilityRole::Edit
    )
}

/// Converts a source-order position into a runtime identifier component.
///
/// A document is bounded well below this limit, so the saturating conversion is
/// a guard rather than an expected path; it exists because a wrapped index
/// would make two elements share an identity.
fn runtime_index(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use anodrel_ui::{UiAccessibilityRole, UiRect};

    use super::{
        AccessibleElement, ClientOrigin, ScreenRect, UIA_APPEND_RUNTIME_ID, accessible_elements,
        control_type, control_type_for, keyboard_focusable, property, runtime_index,
    };

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"go","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"},{"id":"blocked","kind":"action","label":"Unavailable","fontSize":16,"enabled":false,"tone":"accent"}]}}"#;

    /// A deterministic stand-in for host text measurement.
    struct FixedMeasurer;

    impl anodrel_ui::TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, font_size: u16) -> anodrel_ui::UiSize {
            anodrel_ui::UiSize::new(
                text.chars().count() as f32 * f32::from(font_size) * 0.5,
                f32::from(font_size),
            )
        }
    }

    fn elements() -> Vec<AccessibleElement> {
        let document =
            anodrel_ui_document::decode(DOCUMENT).expect("the fixture document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let snapshot = document.accessibility_snapshot(&layout);
        accessible_elements(&snapshot, ClientOrigin::new(100, 50, 1.0))
    }

    #[test]
    fn every_role_maps_to_its_published_control_type() {
        assert_eq!(control_type_for(UiAccessibilityRole::Group), 50_026);
        assert_eq!(control_type_for(UiAccessibilityRole::StaticText), 50_020);
        assert_eq!(control_type_for(UiAccessibilityRole::Button), 50_000);
        assert_eq!(control_type::GROUP, 50_026);
    }

    #[test]
    fn only_actions_are_reachable_by_keyboard() {
        // Assistive technology and the keyboard must agree on what is
        // reachable, so this mirrors the portable focus traversal exactly.
        assert!(keyboard_focusable(UiAccessibilityRole::Button));
        assert!(!keyboard_focusable(UiAccessibilityRole::StaticText));
        assert!(!keyboard_focusable(UiAccessibilityRole::Group));
    }

    #[test]
    fn published_property_identifiers_match_ui_automation() {
        assert_eq!(property::NAME, 30_005);
        assert_eq!(property::CONTROL_TYPE, 30_003);
        assert_eq!(property::IS_ENABLED, 30_010);
        assert_eq!(property::AUTOMATION_ID, 30_011);
        assert_eq!(property::BOUNDING_RECTANGLE, 30_001);
        assert_eq!(property::IS_KEYBOARD_FOCUSABLE, 30_009);
        assert_eq!(property::IS_CONTROL_ELEMENT, 30_016);
        assert_eq!(property::IS_CONTENT_ELEMENT, 30_017);
    }

    #[test]
    fn a_document_maps_to_its_elements_in_source_order() {
        let mapped = elements();
        let ids = mapped
            .iter()
            .map(AccessibleElement::automation_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["root", "heading", "go", "blocked"]);
    }

    #[test]
    fn names_roles_and_enabled_state_come_from_the_snapshot() {
        let mapped = elements();
        let heading = &mapped[1];
        assert_eq!(heading.name(), "Anodrel");
        assert_eq!(heading.control_type(), control_type::TEXT);
        assert!(!heading.keyboard_focusable());
        // Non-interactive is not the same as unavailable. A screen reader
        // announces a disabled element as out of reach, so reporting text as
        // disabled would describe ordinary prose as dimmed.
        assert!(heading.enabled(), "text must not be announced as disabled");
        assert!(mapped[0].enabled(), "a container is not disabled either");

        let go = &mapped[2];
        assert_eq!(go.name(), "Continue");
        assert_eq!(go.control_type(), control_type::BUTTON);
        assert!(go.enabled());
        assert!(go.keyboard_focusable());

        // A disabled action must still be announced, and announced as
        // unavailable rather than omitted.
        let blocked = &mapped[3];
        assert_eq!(blocked.control_type(), control_type::BUTTON);
        assert!(!blocked.enabled());
    }

    #[test]
    fn an_unnamed_container_reports_an_empty_name() {
        // UI Automation has no absent-name representation; empty is what
        // "not named" means there.
        let mapped = elements();
        assert_eq!(mapped[0].control_type(), control_type::GROUP);
        assert_eq!(mapped[0].name(), "");
    }

    #[test]
    fn runtime_identifiers_are_prefixed_and_unique_within_a_snapshot() {
        let mapped = elements();
        let mut ids = mapped
            .iter()
            .map(|element| {
                assert_eq!(element.runtime_id()[0], UIA_APPEND_RUNTIME_ID);
                element.runtime_id()[1]
            })
            .collect::<Vec<_>>();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "two elements shared a runtime identity");
    }

    #[test]
    fn an_index_beyond_the_identifier_range_saturates_rather_than_wrapping() {
        // Wrapping would make two elements share an identity, which is worse
        // than a duplicate at the very end of an impossible document.
        assert_eq!(runtime_index(7), 7);
        assert_eq!(runtime_index(usize::MAX), i32::MAX);
    }

    #[test]
    fn bounds_are_placed_at_the_window_position() {
        let mapped = elements();
        assert_ne!(mapped[1].bounds(), ScreenRect::EMPTY);
        assert!(mapped[1].bounds().left >= 100.0);
        assert!(mapped[1].bounds().top >= 50.0);
    }
}
