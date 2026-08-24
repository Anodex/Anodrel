//! A host-owned visual and input test for `anodrel-ui`.
//!
//! This module is intentionally a diagnostic surface, not an application UI
//! runtime. It renders a fixed document compiled into the host and reports a
//! clicked action ID back into the same host-owned screen. No UI event opens a
//! process, reads a file, sends a protocol message, or grants a capability.

mod accessibility;
mod fixture;
mod input;
mod render;
mod scrolling;

use anodrel_brand::palette;
use anodrel_canvas::{Canvas, Color, Paint, Point, Rect, point};
use anodrel_ui::{
    Action, Axis, ElementId, FIELD_HORIZONTAL_PADDING, Field, Insets, Scroll, Stack, Text,
    TextMeasurer, UiActionTone, UiDocument, UiEvent, UiFieldState, UiFieldStates, UiFocus,
    UiLayout, UiLayoutKind, UiNode, UiPoint, UiRect, UiScrollOffsets, UiScrollState, UiScrollWheel,
    UiSize, UiSurfaceTone, UiTextTone, wrap_text,
};
use anodrel_ui_document::decode;
use anodrel_ui_session::UiFieldSnapshot;
use anodrel_windows_appearance::{Rgb, SystemAppearance, SystemColors};
use anodrel_windows_uia::{
    UiAutomationFocusMailbox, UiAutomationFocusRoute, UiAutomationScrollCommand,
    UiAutomationScrollMailbox, UiAutomationScrollRoute, UiAutomationScrollSnapshot,
};

use super::scrollbar::{Scrollbar, ScrollbarHit};
use super::text;
use super::text::{Align, TextSpec};
use render::{Surface, WindowsTextMeasurer};
#[cfg(test)]
use render::{UiLabPalette, status_text};

/// Draws the UI Lab through its dedicated software-canvas renderer.
pub(super) fn draw(canvas: &mut Canvas, lab: &UiLab) {
    render::draw(canvas, lab);
}

const BASE_WIDTH: f32 = 920.0;
const BASE_HEIGHT: f32 = 660.0;
const WEIGHT_REGULAR: i32 = 400;
const UI_LAB_DOCUMENT_JSON: &str = include_str!("ui_lab_document.json");

/// One editing key applied to a focused field.
///
/// A closed set. Every key the host forwards is named here, so a future key
/// cannot arrive as an opaque code the field logic has to interpret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FieldEdit {
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

/// Returns the field with one element ID, if the document holds it.
#[derive(Clone)]
pub(super) struct UiLab {
    document: UiDocument,
    status_target: Option<ElementId>,
    focus: UiFocus,
    scroll_offsets: UiScrollOffsets,
    wheel: UiScrollWheel,
    scrollbar_drag: Option<ScrollbarDrag>,
    /// A thumb press remains a scrollbar gesture until its matching release,
    /// even if a newer document removes the viewport while it is captured.
    scrollbar_release_pending: bool,
    /// What a person has typed into this view's fields.
    ///
    /// Host-owned and never sent anywhere. A document seeds it; after that only
    /// a person changes it. See `docs/UI_FIELDS.md`.
    fields: UiFieldStates,
    /// The private UI Automation route for this one native view.
    ///
    /// A provider receives only a revision-bound route, never this mutable
    /// view. The UI thread revalidates every request below.
    automation_focus: UiAutomationFocusMailbox,
    /// The private UI Automation route for this view's retained scroll state.
    automation_scroll: UiAutomationScrollMailbox,
    pub(super) hovered: Option<ElementId>,
    pub(super) last_action: Option<ElementId>,
}

/// Private state retained only while Windows has captured one scrollbar thumb.
#[derive(Clone)]
struct ScrollbarDrag {
    id: ElementId,
    grab_offset_y: f32,
}

/// The local result of one UI Automation focus request.
///
/// `accepted` stays true when the requested element was already focused: that
/// is a truthful `SetFocus` success, but it is not a new focus transition to
/// announce to Windows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AccessibilityFocusResult {
    pub(super) accepted: bool,
    pub(super) changed: bool,
}

/// The local result of one UI Automation scroll request.
///
/// An accepted request can already be at its requested position. That remains
/// a truthful UI Automation success without a repaint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AccessibilityScrollResult {
    pub(super) accepted: bool,
    pub(super) changed: bool,
}

#[cfg(test)]
mod tests;
