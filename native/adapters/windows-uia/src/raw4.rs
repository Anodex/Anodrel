//! UI Automation focus-reporting identifiers.
//!
//! Focus remains host-owned. This module names only the property used to
//! report that immutable snapshot; it contains no control interface or event
//! binding. See Decision 0070.

/// `UIA_HasKeyboardFocusPropertyId`.
pub const UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID: i32 = 30_008;

#[cfg(test)]
mod tests {
    use super::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID;

    #[test]
    fn the_focus_property_identifier_matches_ui_automation() {
        assert_eq!(UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID, 30_008);
    }
}
