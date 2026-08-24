//! The validated UI tree and document root.

use super::*;

/// One supported node in a UI document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiNode {
    /// A source-ordered stack.
    Stack(Stack),
    /// A vertically clipped scroll viewport.
    Scroll(Scroll),
    /// A non-interactive text run.
    Text(Text),
    /// One visible semantic status result.
    Status(Status),
    /// A semantic action.
    Action(Action),
    /// A single-line field a person can type into.
    Field(Field),
}

impl UiNode {
    /// Returns this node's validated element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        match self {
            Self::Stack(stack) => stack.id(),
            Self::Scroll(scroll) => scroll.id(),
            Self::Text(text) => text.id(),
            Self::Status(status) => status.id(),
            Self::Action(action) => action.id(),
            Self::Field(field) => field.id(),
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

    /// Returns this document's one semantic status, when it has one.
    ///
    /// Validation guarantees there is never a second result to choose between.
    #[must_use]
    pub fn status(&self) -> Option<&Status> {
        status_in_node(&self.root)
    }
}
