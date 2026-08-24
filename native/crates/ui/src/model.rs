//! Validated, portable UI document data.

use crate::{ElementId, UiActionTone, UiError, UiSurfaceTone, UiTextTone};

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

mod containers;
mod content;
mod tree;
mod validation;

pub use containers::{Scroll, Stack};
pub use content::{
    Action, Field, MAX_FIELD_LENGTH, MIN_FIELD_LENGTH, Status, Text, UiStatusPoliteness,
};
pub use tree::{UiDocument, UiNode};

use validation::*;

#[cfg(test)]
mod tests;
