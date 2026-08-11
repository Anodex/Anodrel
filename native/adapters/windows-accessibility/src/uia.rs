//! The fixed Microsoft UI Automation identifiers this adapter uses.
//!
//! These are stable published constants, listed here rather than pulled from a
//! dependency so the adapter keeps Anodrel's rule of shipping no third-party
//! runtime crate. Only the identifiers the mapping in `docs/ACCESSIBILITY.md`
//! actually supplies appear here; an identifier with no source in the portable
//! snapshot is deliberately absent.

/// UI Automation control type identifiers.
pub mod control_type {
    /// `UIA_ButtonControlTypeId`.
    pub const BUTTON: i32 = 50_000;
    /// `UIA_TextControlTypeId`.
    pub const TEXT: i32 = 50_020;
    /// `UIA_GroupControlTypeId`.
    pub const GROUP: i32 = 50_026;
    /// `UIA_EditControlTypeId`.
    pub const EDIT: i32 = 50_004;
}

/// UI Automation property identifiers.
pub mod property {
    /// `UIA_BoundingRectanglePropertyId`.
    pub const BOUNDING_RECTANGLE: i32 = 30_001;
    /// `UIA_ControlTypePropertyId`.
    pub const CONTROL_TYPE: i32 = 30_003;
    /// `UIA_NamePropertyId`.
    pub const NAME: i32 = 30_005;
    /// `UIA_IsKeyboardFocusablePropertyId`.
    pub const IS_KEYBOARD_FOCUSABLE: i32 = 30_009;
    /// `UIA_IsEnabledPropertyId`.
    pub const IS_ENABLED: i32 = 30_010;
    /// `UIA_AutomationIdPropertyId`.
    pub const AUTOMATION_ID: i32 = 30_011;
    /// `UIA_IsControlElementPropertyId`.
    pub const IS_CONTROL_ELEMENT: i32 = 30_016;
    /// `UIA_IsContentElementPropertyId`.
    pub const IS_CONTENT_ELEMENT: i32 = 30_017;
}

/// Prefix telling Windows to append this provider's runtime ID to the host
/// window's own, so identifiers stay unique across windows without the adapter
/// keeping a process-wide registry.
pub const UIA_APPEND_RUNTIME_ID: i32 = 3;
