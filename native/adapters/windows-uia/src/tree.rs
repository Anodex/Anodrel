//! The element tree one provider publishes, and the pure logic over it.
//!
//! Keeping navigation, property lookup, and hit testing here means the COM
//! layer holds only pointers and reference counts, and every rule about what a
//! client can see is testable without Windows.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

use anodrel_ui::ElementId;
use anodrel_windows_accessibility::{
    AccessibleElement, ScreenRect, control_type, live_setting, property,
};

use crate::raw::{CONTROL_TYPE_WINDOW, Variant};
use crate::raw2::{UiaRect, direction};
use crate::raw4::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID;
use crate::raw5::{UIA_VALUE_IS_READ_ONLY_PROPERTY_ID, UIA_VALUE_VALUE_PROPERTY_ID};
use crate::raw6::{
    UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID, UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID,
    UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID, UIA_SCROLL_PATTERN_NO_SCROLL,
    UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID, UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID,
    UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID,
};
use crate::{
    UiAutomationActionSink, UiAutomationFocusSink, UiAutomationScrollCommand,
    UiAutomationScrollSink, UiAutomationScrollSnapshot,
};

/// The fixed automation identifier for an Anodrel surface's root.
///
/// Host-owned text. An application cannot supply or change it.
pub const ROOT_AUTOMATION_ID: &str = "anodrel.surface";

/// One window's published accessibility tree.
#[derive(Debug)]
pub struct Tree {
    title: Vec<u16>,
    elements: Vec<AccessibleElement>,
    relationships: Relationships,
    field_values: BTreeMap<String, Vec<u16>>,
    /// A provider's initial snapshot, updated only after that same provider's
    /// successful `SetFocus` call. It never observes unrelated live focus.
    focused: Mutex<Option<usize>>,
    action_sink: Option<UiAutomationActionSink>,
    focus_sink: Option<UiAutomationFocusSink>,
    scroll: Option<ScrollCapability>,
}

/// The one host-selected vertical viewport that this immutable tree may expose.
#[derive(Debug)]
struct ScrollCapability {
    snapshot: UiAutomationScrollSnapshot,
    items: BTreeSet<ElementId>,
    sink: UiAutomationScrollSink,
}

/// Immutable direct relationships derived from one mapped semantic snapshot.
///
/// The portable model emits preorder data, so a valid parent is earlier than
/// its child. Rejecting every other value keeps an accidentally malformed
/// mapping bounded and acyclic; it becomes a top-level child rather than a
/// relationship the provider cannot safely navigate.
#[derive(Debug)]
struct Relationships {
    parents: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    root_children: Vec<usize>,
}

impl Relationships {
    fn from_elements(elements: &[AccessibleElement]) -> Self {
        let mut relationships = Self {
            parents: vec![None; elements.len()],
            children: (0..elements.len()).map(|_| Vec::new()).collect(),
            root_children: Vec::new(),
        };

        for (index, element) in elements.iter().enumerate() {
            let parent = element
                .parent_index()
                .filter(|parent| *parent < index && *parent < elements.len());
            relationships.parents[index] = parent;
            match parent {
                Some(parent) => relationships.children[parent].push(index),
                None => relationships.root_children.push(index),
            }
        }
        relationships
    }

    fn step(&self, element: Option<usize>, towards: i32) -> Option<Option<usize>> {
        match element {
            // The root's parent belongs to Windows, not to this provider.
            None => match towards {
                direction::PARENT => None,
                direction::FIRST_CHILD => self.root_children.first().copied().map(Some),
                direction::LAST_CHILD => self.root_children.last().copied().map(Some),
                _ => None,
            },
            Some(index) => {
                let parent = *self.parents.get(index)?;
                match towards {
                    direction::PARENT => Some(parent),
                    direction::FIRST_CHILD => self.children[index].first().copied().map(Some),
                    direction::LAST_CHILD => self.children[index].last().copied().map(Some),
                    direction::NEXT_SIBLING => {
                        let siblings = self.siblings(index, parent)?;
                        let position = siblings.iter().position(|sibling| *sibling == index)?;
                        siblings.get(position + 1).copied().map(Some)
                    }
                    direction::PREVIOUS_SIBLING => {
                        let siblings = self.siblings(index, parent)?;
                        let position = siblings.iter().position(|sibling| *sibling == index)?;
                        position
                            .checked_sub(1)
                            .and_then(|position| siblings.get(position))
                            .copied()
                            .map(Some)
                    }
                    _ => None,
                }
            }
        }
    }

    fn siblings(&self, index: usize, parent: Option<usize>) -> Option<&[usize]> {
        self.parents.get(index)?;
        match parent {
            Some(parent) => self.children.get(parent).map(Vec::as_slice),
            None => Some(&self.root_children),
        }
    }
}

impl Tree {
    /// Builds the tree for one window title and its mapped semantic elements.
    ///
    /// Focus and field values are reduced to visible published targets while
    /// the tree is created, so a provider never reads a mutable view. The
    /// action sink exists only for a current authenticated UI session. The
    /// focus sink is a host-only route for this one view; both are gated per
    /// element below.
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
        let relationships = Relationships::from_elements(&elements);
        let field_values = field_values
            .into_iter()
            .map(|(id, value)| (id.as_str().to_owned(), utf16(&value)))
            .collect();
        Self {
            title,
            elements,
            relationships,
            field_values,
            focused: Mutex::new(focused),
            action_sink,
            focus_sink,
            scroll: None,
        }
    }

    /// Adds the one host-selected scroll capability to this immutable tree.
    ///
    /// The snapshot and route are retained together so a provider can never
    /// advertise a pattern without the host-only path that can revalidate it.
    #[must_use]
    pub(crate) fn with_scroll(
        mut self,
        snapshot: UiAutomationScrollSnapshot,
        items: Vec<ElementId>,
        sink: UiAutomationScrollSink,
    ) -> Self {
        self.scroll = Some(ScrollCapability {
            snapshot,
            items: items.into_iter().collect(),
            sink,
        });
        self
    }

    /// Resolves one direct UI Automation navigation step in this snapshot.
    ///
    /// `None` names the host-owned window root. A `None` result means that no
    /// target exists in the requested direction, which the COM layer reports
    /// as a null provider rather than a failure.
    #[must_use]
    pub fn step(&self, element: Option<usize>, towards: i32) -> Option<Option<usize>> {
        self.relationships.step(element, towards)
    }

    /// Whether this immutable publication has any semantic children.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.elements.is_empty()
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
            property::IS_OFFSCREEN => Some(Variant::boolean(false)),
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
            property::IS_OFFSCREEN => Some(Variant::boolean(!self.is_visible(index))),
            property::IS_KEYBOARD_FOCUSABLE => Some(Variant::boolean(element.keyboard_focusable())),
            property::LIVE_SETTING => Some(Variant::int(element.live_setting())),
            UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID => {
                Some(Variant::boolean(self.focused() == Some(index)))
            }
            UIA_VALUE_VALUE_PROPERTY_ID => Some(Variant::string(self.field_value(index)?)),
            UIA_VALUE_IS_READ_ONLY_PROPERTY_ID => {
                self.field_value(index).map(|_| Variant::boolean(true))
            }
            UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID => self
                .scroll_snapshot(index)
                .map(|_| Variant::double(UIA_SCROLL_PATTERN_NO_SCROLL)),
            UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID => {
                self.scroll_snapshot(index).map(|_| Variant::double(100.0))
            }
            UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID => self
                .scroll_snapshot(index)
                .map(|snapshot| Variant::double(snapshot.vertical_scroll_percent())),
            UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID => self
                .scroll_snapshot(index)
                .map(|snapshot| Variant::double(snapshot.vertical_view_size())),
            UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID => {
                self.scroll_snapshot(index).map(|_| Variant::boolean(false))
            }
            UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID => {
                self.scroll_snapshot(index).map(|_| Variant::boolean(true))
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

    /// Returns the current visible semantic status that may source one
    /// live-region notification. The ID came from the host's comparison of
    /// accepted documents; this method still validates that the fresh provider
    /// contains an on-screen live node before Windows receives it.
    #[must_use]
    pub(crate) fn live_region(&self, id: &ElementId) -> Option<usize> {
        self.elements
            .iter()
            .enumerate()
            .find(|(index, element)| {
                element.automation_id() == id.as_str()
                    && element.live_setting() != live_setting::OFF
                    && self.is_visible(*index)
            })
            .map(|(index, _)| index)
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

    /// Whether one published Group exposes the bounded vertical ScrollPattern.
    #[must_use]
    pub(crate) fn supports_scroll(&self, index: usize) -> bool {
        self.scroll_snapshot(index).is_some()
    }

    /// Whether one published descendant exposes `ScrollItemPattern`.
    ///
    /// The selected viewport itself remains only a Scroll provider. Each
    /// accepted child identity came from the host's nearest-scroll-ancestor
    /// walk, so a provider cannot use this route to select another viewport.
    #[must_use]
    pub(crate) fn supports_scroll_item(&self, index: usize) -> bool {
        self.scroll_item_target(index).is_some()
    }

    /// Returns the published immutable scroll values for one selected group.
    #[must_use]
    pub(crate) fn scroll_snapshot(&self, index: usize) -> Option<&UiAutomationScrollSnapshot> {
        let capability = self.scroll.as_ref()?;
        let element = self.elements.get(index)?;
        (element.control_type() == control_type::GROUP
            && element.automation_id() == capability.snapshot.target().as_str())
        .then_some(&capability.snapshot)
    }

    /// Offers one closed vertical command to the selected host-owned viewport.
    ///
    /// `false` combines every generic refusal. The provider has no route to an
    /// application, a pointer stream, another viewport, or a failure detail.
    pub(crate) fn scroll(&self, index: usize, command: UiAutomationScrollCommand) -> bool {
        let capability = match (self.scroll.as_ref(), self.scroll_snapshot(index)) {
            (Some(capability), Some(_)) => capability,
            _ => return false,
        };
        capability
            .sink
            .scroll(capability.snapshot.target().clone(), command)
    }

    /// Asks the selected viewport to reveal one of its bounded descendants.
    ///
    /// A fully clipped element may use this one route, but it cannot gain
    /// focus, invocation, or a field-value read merely by appearing in the
    /// immutable navigation tree.
    pub(crate) fn scroll_into_view(&self, index: usize) -> bool {
        let Some(target) = self.scroll_item_target(index) else {
            return false;
        };
        let Some(capability) = &self.scroll else {
            return false;
        };
        capability.sink.scroll(
            capability.snapshot.target().clone(),
            UiAutomationScrollCommand::ScrollIntoView { item: target },
        )
    }

    fn invocation_id(&self, index: usize) -> Option<ElementId> {
        let element = self.elements.get(index)?;
        (self.is_visible(index)
            && element.control_type() == control_type::BUTTON
            && element.enabled())
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
        (self.is_visible(index) && element.control_type() == control_type::EDIT)
            .then(|| {
                self.field_values
                    .get(element.automation_id())
                    .map(Vec::as_slice)
            })
            .flatten()
    }

    fn scroll_item_target(&self, index: usize) -> Option<ElementId> {
        let capability = self.scroll.as_ref()?;
        let element = self.elements.get(index)?;
        let id = ElementId::new(element.automation_id()).ok()?;
        (id.as_str() != capability.snapshot.target().as_str() && capability.items.contains(&id))
            .then_some(id)
    }

    fn is_visible(&self, index: usize) -> bool {
        self.elements
            .get(index)
            .is_some_and(|element| !element.bounds().is_empty())
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
mod tests;
