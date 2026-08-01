//! Validated, portable UI document data.

use std::collections::BTreeSet;

use crate::{ElementId, UiError};

/// The maximum number of nodes in one UI document.
pub const MAX_NODES: usize = 512;
/// The maximum root-inclusive nesting depth in one UI document.
pub const MAX_DEPTH: usize = 32;
/// The maximum combined UTF-8 bytes of text and action labels in one document.
pub const MAX_TEXT_BYTES: usize = 32 * 1024;
/// The smallest supported font size in logical pixels.
pub const MIN_FONT_SIZE: u16 = 8;
/// The largest supported font size in logical pixels.
pub const MAX_FONT_SIZE: u16 = 96;
/// The largest supported padding or gap in logical pixels.
pub const MAX_SPACING: u16 = 256;

/// A stack's primary placement direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Place children from top to bottom.
    Vertical,
    /// Place children from left to right.
    Horizontal,
}

/// Validated padding in logical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Insets {
    pub(crate) left: u16,
    pub(crate) top: u16,
    pub(crate) right: u16,
    pub(crate) bottom: u16,
}

impl Insets {
    /// Builds validated padding values.
    pub fn new(left: u16, top: u16, right: u16, bottom: u16) -> Result<Self, UiError> {
        if [left, top, right, bottom]
            .into_iter()
            .any(|value| value > MAX_SPACING)
        {
            return Err(UiError::InvalidSpacing);
        }
        Ok(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Builds equal padding on every side.
    pub fn all(value: u16) -> Result<Self, UiError> {
        Self::new(value, value, value, value)
    }

    /// Builds zero padding.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    /// Returns left padding.
    #[must_use]
    pub const fn left(self) -> u16 {
        self.left
    }

    /// Returns top padding.
    #[must_use]
    pub const fn top(self) -> u16 {
        self.top
    }

    /// Returns right padding.
    #[must_use]
    pub const fn right(self) -> u16 {
        self.right
    }

    /// Returns bottom padding.
    #[must_use]
    pub const fn bottom(self) -> u16 {
        self.bottom
    }
}

/// A source-ordered stack of child nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stack {
    pub(crate) id: ElementId,
    pub(crate) axis: Axis,
    pub(crate) padding: Insets,
    pub(crate) gap: u16,
    pub(crate) children: Vec<UiNode>,
}

impl Stack {
    /// Builds a stack with bounded padding and inter-child gap.
    pub fn new(
        id: ElementId,
        axis: Axis,
        padding: Insets,
        gap: u16,
        children: Vec<UiNode>,
    ) -> Result<Self, UiError> {
        if gap > MAX_SPACING {
            return Err(UiError::InvalidSpacing);
        }
        Ok(Self {
            id,
            axis,
            padding,
            gap,
            children,
        })
    }

    /// Returns this stack's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns its placement direction.
    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }

    /// Returns its padding.
    #[must_use]
    pub const fn padding(&self) -> Insets {
        self.padding
    }

    /// Returns its inter-child gap.
    #[must_use]
    pub const fn gap(&self) -> u16 {
        self.gap
    }

    /// Returns child nodes in source order.
    #[must_use]
    pub fn children(&self) -> &[UiNode] {
        &self.children
    }
}

/// A non-interactive, single-line text run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text {
    pub(crate) id: ElementId,
    pub(crate) value: String,
    pub(crate) font_size: u16,
}

impl Text {
    /// Builds validated plain text with a logical-pixel font size.
    pub fn new(id: ElementId, value: impl Into<String>, font_size: u16) -> Result<Self, UiError> {
        let value = value.into();
        validate_text(&value)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            value,
            font_size,
        })
    }

    /// Returns this text run's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated plain-text value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }
}

/// A semantic, optionally enabled action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    pub(crate) id: ElementId,
    pub(crate) label: String,
    pub(crate) font_size: u16,
    pub(crate) enabled: bool,
}

impl Action {
    /// Builds a validated semantic action.
    ///
    /// Its [`ElementId`] is the only identity reported by action hit testing;
    /// this type contains no command, callback, or native operation.
    pub fn new(
        id: ElementId,
        label: impl Into<String>,
        font_size: u16,
        enabled: bool,
    ) -> Result<Self, UiError> {
        let label = label.into();
        validate_text(&label)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            label,
            font_size,
            enabled,
        })
    }

    /// Returns this action's semantic element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated visible label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }

    /// Returns whether this action participates in hit testing.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// One supported node in a UI document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiNode {
    /// A source-ordered stack.
    Stack(Stack),
    /// A non-interactive text run.
    Text(Text),
    /// A semantic action.
    Action(Action),
}

impl UiNode {
    /// Returns this node's validated element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        match self {
            Self::Stack(stack) => stack.id(),
            Self::Text(text) => text.id(),
            Self::Action(action) => action.id(),
        }
    }
}

/// One fully validated in-memory UI tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDocument {
    root: UiNode,
}

impl UiDocument {
    /// Validates a root node and builds a document.
    ///
    /// This validates document-wide node, depth, text-budget, and unique-ID
    /// limits. It does not parse an application file or accept untrusted input.
    pub fn new(root: UiNode) -> Result<Self, UiError> {
        let mut validator = DocumentValidator::default();
        validator.visit(&root, 1)?;
        Ok(Self { root })
    }

    /// Returns the immutable, validated root node.
    #[must_use]
    pub fn root(&self) -> &UiNode {
        &self.root
    }
}

fn validate_text(value: &str) -> Result<(), UiError> {
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        Err(UiError::InvalidText)
    } else {
        Ok(())
    }
}

fn validate_font_size(font_size: u16) -> Result<(), UiError> {
    if (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&font_size) {
        Ok(())
    } else {
        Err(UiError::InvalidFontSize)
    }
}

#[derive(Default)]
struct DocumentValidator {
    ids: BTreeSet<ElementId>,
    node_count: usize,
    text_bytes: usize,
}

impl DocumentValidator {
    fn visit(&mut self, node: &UiNode, depth: usize) -> Result<(), UiError> {
        if depth > MAX_DEPTH {
            return Err(UiError::DepthLimitExceeded);
        }
        self.node_count += 1;
        if self.node_count > MAX_NODES {
            return Err(UiError::NodeLimitExceeded);
        }
        if !self.ids.insert(node.id().clone()) {
            return Err(UiError::DuplicateElementId);
        }

        match node {
            UiNode::Stack(stack) => {
                for child in &stack.children {
                    self.visit(child, depth + 1)?;
                }
            }
            UiNode::Text(text) => self.add_text(text.value.len())?,
            UiNode::Action(action) => self.add_text(action.label.len())?,
        }
        Ok(())
    }

    fn add_text(&mut self, bytes: usize) -> Result<(), UiError> {
        self.text_bytes = self
            .text_bytes
            .checked_add(bytes)
            .ok_or(UiError::TextLimitExceeded)?;
        if self.text_bytes > MAX_TEXT_BYTES {
            Err(UiError::TextLimitExceeded)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: impl Into<String>) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn text(id_value: impl Into<String>, value: impl Into<String>) -> UiNode {
        UiNode::Text(Text::new(id(id_value), value, 10).expect("test text is valid"))
    }

    fn stack(id_value: impl Into<String>, child: UiNode) -> UiNode {
        UiNode::Stack(
            Stack::new(id(id_value), Axis::Vertical, Insets::zero(), 0, vec![child])
                .expect("test stack is valid"),
        )
    }

    #[test]
    fn rejects_a_document_deeper_than_the_limit() {
        let mut node = text("leaf", "content");
        for index in 0..MAX_DEPTH {
            node = stack(format!("stack-{index}"), node);
        }
        assert_eq!(UiDocument::new(node), Err(UiError::DepthLimitExceeded));
    }

    #[test]
    fn rejects_a_document_with_more_than_the_node_limit() {
        let children = (0..MAX_NODES)
            .map(|index| text(format!("item-{index}"), "x"))
            .collect();
        let root = UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        );
        assert_eq!(UiDocument::new(root), Err(UiError::NodeLimitExceeded));
    }

    #[test]
    fn rejects_text_that_exceeds_the_document_budget() {
        let root = UiNode::Stack(
            Stack::new(
                id("root"),
                Axis::Vertical,
                Insets::zero(),
                0,
                vec![
                    text("full", "x".repeat(MAX_TEXT_BYTES)),
                    text("one-more", "x"),
                ],
            )
            .expect("test stack is valid"),
        );
        assert_eq!(UiDocument::new(root), Err(UiError::TextLimitExceeded));
    }
}
