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
mod field_state;
mod focus;
mod geometry;
mod id;
mod layout;
mod model;
mod scroll;
mod text_wrap;

pub use accessibility::{
    UiAccessibilityLiveSetting, UiAccessibilityNode, UiAccessibilityRole, UiAccessibilitySnapshot,
};
pub use appearance::{UiActionTone, UiSurfaceTone, UiTextTone};
pub use error::UiError;
pub use field_state::{UiFieldState, UiFieldStates};
pub use focus::UiFocus;
pub use geometry::{UiPoint, UiRect, UiSize};
pub use id::ElementId;
pub use layout::{
    ACTION_HORIZONTAL_PADDING, ACTION_MINIMUM_HEIGHT, ACTION_VERTICAL_PADDING,
    FIELD_HORIZONTAL_PADDING, FIELD_MINIMUM_HEIGHT, FIELD_VERTICAL_PADDING, TextMeasurer, UiEvent,
    UiLayout, UiLayoutItem, UiLayoutKind, UiScrollMetrics, UiScrollOffsets,
};
pub use model::{
    Action, Axis, Field, Insets, MAX_FIELD_LENGTH, MIN_FIELD_LENGTH, Scroll, Stack, Status, Text,
    UiDocument, UiNode, UiStatusPoliteness,
};
pub use scroll::{DEFAULT_SCROLL_LINE, UiScrollState, UiScrollWheel, WHEEL_DELTA_PER_LINE};
pub use text_wrap::{MAX_TEXT_LINES, wrap_text, wrapped_height};
