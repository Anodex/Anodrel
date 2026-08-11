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
    /// Named by its label. Its value is deliberately absent: see
    /// `docs/UI_FIELDS.md` and `docs/ACCESSIBILITY.md`, which together mean
    /// assistive technology can find a field and cannot read what is in it.
    Edit,
}

/// One accessible semantic element from a visible layout pass.
#[derive(Clone, Debug, PartialEq)]
pub struct UiAccessibilityNode {
    id: ElementId,
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

    /// Returns whether this element can be invoked.
    ///
    /// Only buttons can be enabled. An enabled button remains semantic data;
    /// this snapshot cannot invoke it or grant it native authority.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// A source-ordered snapshot of the visible UI accessibility semantics.
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
        collect_node(self.root(), &visible_bounds, &mut snapshot.nodes);
        snapshot
    }
}

fn collect_node(
    node: &UiNode,
    visible_bounds: &BTreeMap<ElementId, UiRect>,
    output: &mut Vec<UiAccessibilityNode>,
) {
    let Some(bounds) = visible_bounds.get(node.id()).copied() else {
        return;
    };
    let (role, name, enabled) = semantic_fields(node);
    output.push(UiAccessibilityNode {
        id: node.id().clone(),
        role,
        name,
        bounds,
        enabled,
    });
    match node {
        UiNode::Stack(stack) => {
            for child in stack.children() {
                collect_node(child, visible_bounds, output);
            }
        }
        UiNode::Scroll(scroll) => collect_node(scroll.child(), visible_bounds, output),
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

/// A field is announced by its label and never by what has been typed into it.
///
/// The accessibility snapshot is a published surface, so putting the value here
/// would hand it to anything reading the tree — including a path an application
/// could reach later. The value leaves the host only through the granted
/// snapshot of Decision 0067, and this is the other door it must not leave by.
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
        assert_eq!(snapshot.nodes()[0].name(), None);
        assert_eq!(snapshot.nodes()[1].id().as_str(), "welcome");
        assert_eq!(snapshot.nodes()[1].role(), UiAccessibilityRole::StaticText);
        assert_eq!(snapshot.nodes()[1].name(), Some("Welcome"));
        assert_eq!(snapshot.nodes()[2].role(), UiAccessibilityRole::Button);
        assert_eq!(snapshot.nodes()[2].name(), Some("Continue"));
        assert!(!snapshot.nodes()[2].enabled());
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
