//! Anodrel's owned native UI foundation.
//!
//! The crate defines a small, portable in-memory view tree, deterministic
//! layout, clipping, and semantic action hit testing. It has no renderer,
//! operating-system calls, package format, protocol, scripting surface, or
//! third-party dependency. A host supplies only text measurement; a later
//! adapter may render the resulting layout and route its semantic events.
//!
//! See `docs/UI.md` for the public contract and Decision 0025 for the
//! boundary that keeps this layer separate from applications and native
//! authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod accessibility;
mod appearance;
mod error;
mod focus;
mod geometry;
mod id;
mod layout;
mod model;
mod scroll;

pub use accessibility::{UiAccessibilityNode, UiAccessibilityRole, UiAccessibilitySnapshot};
pub use appearance::{UiActionTone, UiSurfaceTone, UiTextTone};
pub use error::UiError;
pub use focus::UiFocus;
pub use geometry::{UiPoint, UiRect, UiSize};
pub use id::ElementId;
pub use layout::{
    ACTION_HORIZONTAL_PADDING, ACTION_MINIMUM_HEIGHT, ACTION_VERTICAL_PADDING, TextMeasurer,
    UiEvent, UiLayout, UiLayoutItem, UiLayoutKind, UiScrollMetrics, UiScrollOffsets,
};
pub use model::{Action, Axis, Insets, Scroll, Stack, Text, UiDocument, UiNode};
pub use scroll::{DEFAULT_SCROLL_LINE, UiScrollState};
