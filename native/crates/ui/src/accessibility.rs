//! Portable accessibility semantics for a visible UI layout.

use std::collections::BTreeMap;

use crate::{Action, ElementId, Field, Text, UiDocument, UiLayout, UiNode, UiRect};

/// The semantic role of a visible UI node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAccessibilityRole {
    /// A non-interactive container for related child nodes.
    Group,
    /// A non-interactive text value.
    StaticText,
    /// A semantic action that may be enabled or disabled.
    Button,
    /// A single-line field a person can type into.
    ///
    /// Named by its label. Its current value is deliberately absent from this
    /// portable snapshot: the Windows provider separately copies host-owned
    /// field state into its narrow read-only Value pattern (Decision 0071).
    Edit,
}

/// One accessible semantic element from a visible layout pass.
#[derive(Clone, Debug, PartialEq)]
pub struct UiAccessibilityNode {
    id: ElementId,
    parent_index: Option<usize>,
    role: UiAccessibilityRole,
    name: Option<String>,
    bounds: UiRect,
    enabled: bool,
}

impl UiAccessibilityNode {
    /// Returns the stable document element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns this node's direct visible parent's source-order index.
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

    /// Returns the layout's clipped visible bounds in logical pixels.
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
}

/// A source-ordered preorder snapshot of the visible UI accessibility semantics.
///
/// Each node carries its direct visible parent's earlier source-order index, so
/// an operating-system adapter can preserve the document's declared hierarchy
/// without inspecting pixels or a mutable host view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiAccessibilitySnapshot {
    nodes: Vec<UiAccessibilityNode>,
}

impl UiAccessibilitySnapshot {
    /// Returns visible semantic nodes in document source order.
    #[must_use]
    pub fn nodes(&self) -> &[UiAccessibilityNode] {
        &self.nodes
    }
}

impl UiDocument {
    /// Builds accessibility semantics for one specific visible layout pass.
    ///
    /// Only nodes with non-empty clipped layout bounds are included. The
    /// result contains no operating-system object, focus state, keyboard
    /// navigation, live announcement, or action invocation mechanism.
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
    let (role, name, enabled) = semantic_fields(node);
    let index = output.len();
    output.push(UiAccessibilityNode {
        id: node.id().clone(),
        parent_index,
        role,
        name,
        bounds,
        enabled,
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
        UiNode::Text(_) | UiNode::Action(_) | UiNode::Field(_) => {}
    }
}

fn semantic_fields(node: &UiNode) -> (UiAccessibilityRole, Option<String>, bool) {
    match node {
        UiNode::Stack(_) | UiNode::Scroll(_) => (UiAccessibilityRole::Group, None, false),
        UiNode::Text(text) => text_fields(text),
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
fn field_fields(field: &Field) -> (UiAccessibilityRole, Option<String>, bool) {
    (
        UiAccessibilityRole::Edit,
        Some(field.label().to_owned()),
        field.enabled(),
    )
}

fn text_fields(text: &Text) -> (UiAccessibilityRole, Option<String>, bool) {
    (
        UiAccessibilityRole::StaticText,
        Some(text.value().to_owned()),
        false,
    )
}

fn action_fields(action: &Action) -> (UiAccessibilityRole, Option<String>, bool) {
    (
        UiAccessibilityRole::Button,
        Some(action.label().to_owned()),
        action.enabled(),
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        Action, Axis, ElementId, Insets, Stack, Text, TextMeasurer, UiDocument, UiNode, UiRect,
        UiSize,
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
    fn excludes_fully_clipped_nodes() {
        let document = UiDocument::new(stack(vec![action("first", true), action("second", true)]))
            .expect("document is valid");
        let layout = document.layout(UiRect::from_size(0.0, 0.0, 200.0, 36.0), &FixedMeasurer);
        let snapshot = document.accessibility_snapshot(&layout);
        let ids = snapshot
            .nodes()
            .iter()
            .map(|node| node.id().as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["root", "first"]);
    }
}
