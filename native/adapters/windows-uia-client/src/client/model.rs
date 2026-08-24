//! Closed data types exposed by the host-only automation client.

use crate::{com::Com, raw};

/// The fixed properties a host diagnostic may read from one UI Automation node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAutomationNode {
    /// The published user-facing name, empty when the semantic role has none.
    pub name: String,
    /// The published semantic document identifier.
    pub automation_id: String,
    /// The published UI Automation control-type identifier.
    pub control_type: i32,
}

/// One read-only Value-pattern snapshot from a fixed host diagnostic field.
///
/// This data stays inside the host diagnostic adapter. It is not an
/// application protocol result and carries no field selector, write operation,
/// or live subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAutomationValue {
    /// The field's copied UTF-16 value.
    pub value: String,
    /// Whether UI Automation may write the value.
    pub is_read_only: bool,
}

/// One owned immutable UI Automation element interface.
///
/// It deliberately exposes no raw pointer, COM operation, pattern, or mutable
/// state. Only `UiAutomationClient` can read its closed diagnostic values.
pub struct UiAutomationElement {
    pub(super) raw: Com<raw::Element>,
}
