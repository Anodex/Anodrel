//! Container nodes for the validated UI document tree.

use super::*;

/// A source-ordered stack of child nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stack {
    pub(crate) id: ElementId,
    pub(crate) axis: Axis,
    pub(crate) padding: Insets,
    pub(crate) gap: u16,
    pub(crate) surface_tone: UiSurfaceTone,
    pub(crate) children: Vec<UiNode>,
}

/// One vertical viewport with exactly one scrollable child tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scroll {
    pub(crate) id: ElementId,
    pub(crate) child: Box<UiNode>,
}

impl Scroll {
    /// Builds one scroll viewport around a validated child tree.
    #[must_use]
    pub fn new(id: ElementId, child: UiNode) -> Self {
        Self {
            id,
            child: Box::new(child),
        }
    }

    /// Returns this viewport's stable element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns its one source-ordered child tree.
    #[must_use]
    pub fn child(&self) -> &UiNode {
        &self.child
    }
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
            surface_tone: UiSurfaceTone::default(),
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

    /// Returns the requested host-rendered surface treatment.
    #[must_use]
    pub const fn surface_tone(&self) -> UiSurfaceTone {
        self.surface_tone
    }

    /// Requests a host-rendered surface treatment for this stack.
    ///
    /// This is semantic presentation data only. It cannot affect layout,
    /// input, accessibility, or native authority.
    #[must_use]
    pub fn with_surface_tone(mut self, surface_tone: UiSurfaceTone) -> Self {
        self.surface_tone = surface_tone;
        self
    }

    /// Returns child nodes in source order.
    #[must_use]
    pub fn children(&self) -> &[UiNode] {
        &self.children
    }
}
