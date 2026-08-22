//! Portable accessibility semantics for one bounded UI layout.

use std::collections::BTreeMap;

use crate::{
    Action, ElementId, Field, Status, Text, UiDocument, UiLayout, UiNode, UiRect,
    UiStatusPoliteness,
};

/// The semantic role of a UI node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAccessibilityRole {
    /// A non-interactive container for related child nodes.
    Group,
    /// A non-interactive text value.
    StaticText,
    /// One visible semantic status result.
    Status,
    /// A semantic action that may be enabled or disabled.
    Button,
    /// A single-line field a person can type into.
    ///
    /// Named by its label. Its current value is deliberately absent from this
    /// portable snapshot: the Windows provider separately copies host-owned
    /// field state into its narrow read-only Value pattern (Decision 0071).
    Edit,
}

/// The live-setting semantics one accessible node declares.
///
/// This is portable semantic data. A native accessibility adapter maps it to
/// its own fixed property vocabulary; it has no listener, recipient, or
/// delivery result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAccessibilityLiveSetting {
    /// Ordinary nodes are not live regions.
    Off,
    /// A later visible status update is politely announced when an adapter
    /// supports that bounded behavior.
    Polite,
    /// A later visible urgent status update is assertively announced when an
    /// adapter supports that bounded behavior.
    Assertive,
}

/// One accessible semantic element from a layout pass.
#[derive(Clone, Debug, PartialEq)]
pub struct UiAccessibilityNode {
    id: ElementId,
    parent_index: Option<usize>,
    role: UiAccessibilityRole,
    name: Option<String>,
    bounds: UiRect,
    enabled: bool,
    live_setting: UiAccessibilityLiveSetting,
}

impl UiAccessibilityNode {
    /// Returns the stable document element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns this node's direct semantic parent's source-order index.
    ///
    /// `None` means this node belongs directly to the host-owned accessibility
    /// root. Every parent index is earlier than this node because snapshots are
    /// emitted as a preorder walk of the validated document.
    #[must_use]
    pub const fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }

    /// Returns the node's semantic role.
    #[must_use]
    pub const fn role(&self) -> UiAccessibilityRole {
        self.role
    }

    /// Returns the plain-text accessible name, when this role has one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the layout's clipped bounds in logical pixels.
    ///
    /// A wholly clipped node remains in the bounded tree with an empty
    /// rectangle, allowing an operating-system adapter to navigate to it
    /// without claiming that it is visible or locally interactive.
    #[must_use]
    pub const fn bounds(&self) -> UiRect {
        self.bounds
    }

    /// Returns whether this element is enabled for a person's interaction.
    ///
    /// Buttons and fields can be disabled. Enabled state remains semantic data;
    /// this snapshot cannot invoke, edit, or grant native authority.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the semantic live setting for this node.
    #[must_use]
    pub const fn live_setting(&self) -> UiAccessibilityLiveSetting {
        self.live_setting
    }
}

/// A source-ordered preorder snapshot of bounded UI accessibility semantics.
///
/// Each node carries its direct semantic parent's earlier source-order index, so
/// an operating-system adapter can preserve the document's declared hierarchy
/// without inspecting pixels or a mutable host view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiAccessibilitySnapshot {
    nodes: Vec<UiAccessibilityNode>,
}

impl UiAccessibilitySnapshot {
    /// Returns semantic nodes in document source order.
    #[must_use]
    pub fn nodes(&self) -> &[UiAccessibilityNode] {
        &self.nodes
    }
}

impl UiDocument {
    /// Builds accessibility semantics for one specific layout pass.
    ///
    /// Every bounded layout item remains in source order. A node whose clipped
    /// bounds are empty is present with that empty rectangle, but this result
    /// contains no operating-system object, focus state, keyboard navigation,
    /// live announcement, or action invocation mechanism.
    #[must_use]
    pub fn accessibility_snapshot(&self, layout: &UiLayout) -> UiAccessibilitySnapshot {
        let visible_bounds = layout
            .items()
            .iter()
            .map(|item| (item.id().clone(), item.bounds()))
            .collect::<BTreeMap<_, _>>();
        let mut snapshot = UiAccessibilitySnapshot::default();
        collect_node(self.root(), None, &visible_bounds, &mut snapshot.nodes);
        snapshot
    }
}

fn collect_node(
    node: &UiNode,
    parent_index: Option<usize>,
    visible_bounds: &BTreeMap<ElementId, UiRect>,
    output: &mut Vec<UiAccessibilityNode>,
) {
    let Some(bounds) = visible_bounds.get(node.id()).copied() else {
        return;
    };
    let (role, name, enabled, live_setting) = semantic_fields(node);
    let index = output.len();
    output.push(UiAccessibilityNode {
        id: node.id().clone(),
        parent_index,
        role,
        name,
        bounds,
        enabled,
        live_setting,
    });
    match node {
        UiNode::Stack(stack) => {
            for child in stack.children() {
                collect_node(child, Some(index), visible_bounds, output);
            }
        }
        UiNode::Scroll(scroll) => {
            collect_node(scroll.child(), Some(index), visible_bounds, output);
        }
        UiNode::Text(_) | UiNode::Status(_) | UiNode::Action(_) | UiNode::Field(_) => {}
    }
}

fn semantic_fields(
    node: &UiNode,
) -> (
    UiAccessibilityRole,
    Option<String>,
    bool,
    UiAccessibilityLiveSetting,
) {
    match node {
        UiNode::Stack(_) | UiNode::Scroll(_) => (
            UiAccessibilityRole::Group,
            None,
            false,
            UiAccessibilityLiveSetting::Off,
        ),
        UiNode::Text(text) => text_fields(text),
        UiNode::Status(status) => status_fields(status),
        UiNode::Action(action) => action_fields(action),
        UiNode::Field(field) => field_fields(field),
    }
}

/// A field is named by its label; its current text stays out of the portable
/// semantic snapshot.
///
/// The Windows provider separately accepts a host-owned value snapshot above
/// this layer, so no portable document or application protocol path can infer
/// the live field text. See Decisions 0067 and 0071.
fn field_fields(
    field: &Field,
) -> (
    UiAccessibilityRole,
    Option<String>,
    bool,
    UiAccessibilityLiveSetting,
) {
    (
        UiAccessibilityRole::Edit,
        Some(field.label().to_owned()),
        field.enabled(),
        UiAccessibilityLiveSetting::Off,
    )
}

fn text_fields(
    text: &Text,
) -> (
    UiAccessibilityRole,
    Option<String>,
    bool,
    UiAccessibilityLiveSetting,
) {
    (
        UiAccessibilityRole::StaticText,
        Some(text.value().to_owned()),
        false,
        UiAccessibilityLiveSetting::Off,
    )
}

fn status_fields(
    status: &Status,
) -> (
    UiAccessibilityRole,
    Option<String>,
    bool,
    UiAccessibilityLiveSetting,
) {
    (
        UiAccessibilityRole::Status,
        Some(status.value().to_owned()),
        false,
        match status.politeness() {
            UiStatusPoliteness::Polite => UiAccessibilityLiveSetting::Polite,
            UiStatusPoliteness::Assertive => UiAccessibilityLiveSetting::Assertive,
        },
    )
}

fn action_fields(
    action: &Action,
) -> (
    UiAccessibilityRole,
    Option<String>,
    bool,
    UiAccessibilityLiveSetting,
) {
    (
        UiAccessibilityRole::Button,
        Some(action.label().to_owned()),
        action.enabled(),
        UiAccessibilityLiveSetting::Off,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        Action, Axis, ElementId, Insets, Stack, Status, Text, TextMeasurer, UiDocument, UiNode,
        UiRect, UiSize, UiStatusPoliteness,
    };

    use super::UiAccessibilityRole;

    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn measure(&self, _: &str, font_size: u16) -> UiSize {
            UiSize::new(20.0, f32::from(font_size))
        }
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn action(id_value: &str, enabled: bool) -> UiNode {
        UiNode::Action(
            Action::new(id(id_value), "Continue", 10, enabled).expect("test action is valid"),
        )
    }

    fn stack(children: Vec<UiNode>) -> UiNode {
        UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        )
    }

    #[test]
    fn reflects_visible_roles_names_and_enabled_state_in_source_order() {
        let document = UiDocument::new(stack(vec![
            UiNode::Text(Text::new(id("welcome"), "Welcome", 10).expect("text is valid")),
            action("continue", false),
        ]))
        .expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 200.0, 100.0), &FixedMeasurer);
        let snapshot = document.accessibility_snapshot(&layout);

        assert_eq!(snapshot.nodes().len(), 3);
        assert_eq!(snapshot.nodes()[0].role(), UiAccessibilityRole::Group);
        assert_eq!(snapshot.nodes()[0].parent_index(), None);
        assert_eq!(snapshot.nodes()[0].name(), None);
        assert_eq!(snapshot.nodes()[1].id().as_str(), "welcome");
        assert_eq!(snapshot.nodes()[1].parent_index(), Some(0));
        assert_eq!(snapshot.nodes()[1].role(), UiAccessibilityRole::StaticText);
        assert_eq!(snapshot.nodes()[1].name(), Some("Welcome"));
        assert_eq!(snapshot.nodes()[2].role(), UiAccessibilityRole::Button);
        assert_eq!(snapshot.nodes()[2].name(), Some("Continue"));
        assert!(!snapshot.nodes()[2].enabled());
        assert_eq!(snapshot.nodes()[2].parent_index(), Some(0));
    }

    #[test]
    fn preserves_fully_clipped_nodes_with_empty_bounds() {
        let document = UiDocument::new(stack(vec![action("first", true), action("second", true)]))
            .expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 200.0, 36.0), &FixedMeasurer);
        let snapshot = document.accessibility_snapshot(&layout);
        let ids = snapshot
            .nodes()
            .iter()
            .map(|node| node.id().as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["root", "first", "second"]);
        assert!(snapshot.nodes()[2].bounds().is_empty());
        assert_eq!(snapshot.nodes()[2].parent_index(), Some(0));
    }

    #[test]
    fn status_carries_its_semantic_live_setting_without_an_event_mechanism() {
        let document = UiDocument::new(UiNode::Status(
            Status::new(id("status"), "Saved", 12, UiStatusPoliteness::Assertive)
                .expect("status is valid"),
        ))
        .expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 200.0, 100.0), &FixedMeasurer);
        let snapshot = document.accessibility_snapshot(&layout);
        let status = &snapshot.nodes()[0];

        assert_eq!(status.role(), UiAccessibilityRole::Status);
        assert_eq!(status.name(), Some("Saved"));
        assert_eq!(
            status.live_setting(),
            super::UiAccessibilityLiveSetting::Assertive
        );
    }
}
