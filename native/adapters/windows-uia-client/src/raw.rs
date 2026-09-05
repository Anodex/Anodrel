//! Exact ABI declarations for the narrow Windows UI Automation client slice.
//!
//! The private facade preserves one narrow `raw` namespace for the client while
//! keeping constants, value layouts, COM vtables, and system entry points in
//! separately reviewable modules.

#![allow(non_snake_case)]

mod constants;
mod functions;
mod interfaces;
mod values;

pub(crate) use constants::*;
pub(crate) use functions::*;
pub(crate) use interfaces::*;
pub(crate) use values::*;

#[cfg(test)]
mod tests {
    use super::{
        AutomationVtable, ElementVtable, IID_I_UI_AUTOMATION_EVENT_HANDLER,
        IID_I_UI_AUTOMATION_FOCUS_CHANGED_EVENT_HANDLER, IID_I_UI_AUTOMATION_INVOKE_PATTERN,
        IID_I_UI_AUTOMATION_STRUCTURE_CHANGED_EVENT_HANDLER, IID_I_UI_AUTOMATION_VALUE_PATTERN,
        InvokePatternVtable, Point, Rect, STRUCTURE_CHANGE_CHILDREN_INVALIDATED,
        TREE_SCOPE_ELEMENT, TREE_SCOPE_SUBTREE, UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID,
        UIA_INVOKE_PATTERN_ID, UIA_LIVE_REGION_CHANGED_EVENT_ID, UIA_VALUE_PATTERN_ID, VT_BOOL,
        ValuePatternVtable, Variant,
    };

    #[test]
    fn point_and_rectangle_match_the_windows_target_abi() {
        assert_eq!(core::mem::size_of::<Point>(), 8);
        assert_eq!(core::mem::align_of::<Point>(), 4);
        assert_eq!(core::mem::size_of::<Rect>(), 16);
        assert_eq!(core::mem::align_of::<Rect>(), 4);
    }

    #[test]
    fn client_methods_keep_their_windows_sdk_vtable_slots() {
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, element_from_point),
            7 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, get_focused_element),
            8 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(ElementVtable, set_focus),
            3 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(ElementVtable, current_bounding_rectangle),
            43 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(ElementVtable, current_pattern_as),
            14 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(ElementVtable, current_pattern),
            16 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, add_focus_changed_event_handler),
            39 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, add_automation_event_handler),
            32 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, remove_automation_event_handler),
            33 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, remove_focus_changed_event_handler),
            40 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, add_structure_changed_event_handler),
            37 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::offset_of!(AutomationVtable, remove_structure_changed_event_handler),
            38 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::size_of::<ValuePatternVtable>(),
            8 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(
            core::mem::size_of::<InvokePatternVtable>(),
            4 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(UIA_INVOKE_PATTERN_ID, 10_000);
        assert_eq!(UIA_VALUE_PATTERN_ID, 10_002);
        assert_eq!(UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID, 30_008);
        assert_eq!(VT_BOOL, 11);
        assert_eq!(TREE_SCOPE_ELEMENT, 1);
        assert_eq!(TREE_SCOPE_SUBTREE, 7);
        assert_eq!(UIA_LIVE_REGION_CHANGED_EVENT_ID, 20_024);
        assert_eq!(STRUCTURE_CHANGE_CHILDREN_INVALIDATED, 2);
        assert_eq!(
            IID_I_UI_AUTOMATION_FOCUS_CHANGED_EVENT_HANDLER.data1,
            0xc270_f6b5
        );
        assert_eq!(
            IID_I_UI_AUTOMATION_FOCUS_CHANGED_EVENT_HANDLER.data4,
            [0x97, 0x45, 0x7a, 0x7f, 0x97, 0x16, 0x94, 0x68]
        );
        assert_eq!(IID_I_UI_AUTOMATION_EVENT_HANDLER.data1, 0x146c_3c17);
        assert_eq!(IID_I_UI_AUTOMATION_EVENT_HANDLER.data2, 0xf12e);
        assert_eq!(IID_I_UI_AUTOMATION_EVENT_HANDLER.data3, 0x4e22);
        assert_eq!(
            IID_I_UI_AUTOMATION_EVENT_HANDLER.data4,
            [0x8c, 0x27, 0xf8, 0x94, 0xb9, 0xb7, 0x9c, 0x69]
        );
        assert_eq!(
            IID_I_UI_AUTOMATION_STRUCTURE_CHANGED_EVENT_HANDLER.data1,
            0xe81d_1b4e
        );
        assert_eq!(
            IID_I_UI_AUTOMATION_STRUCTURE_CHANGED_EVENT_HANDLER.data4,
            [0x97, 0x54, 0xe7, 0x03, 0x6c, 0x79, 0xf0, 0x54]
        );
        assert_eq!(IID_I_UI_AUTOMATION_INVOKE_PATTERN.data1, 0xfb37_7fbe);
        assert_eq!(
            IID_I_UI_AUTOMATION_INVOKE_PATTERN.data4,
            [0x9c, 0x73, 0x64, 0x99, 0x64, 0x2d, 0x30, 0x59]
        );
        assert_eq!(IID_I_UI_AUTOMATION_VALUE_PATTERN.data1, 0xa94c_d8b1);
        assert_eq!(
            IID_I_UI_AUTOMATION_VALUE_PATTERN.data4,
            [0x9d, 0x2d, 0x64, 0x05, 0x37, 0xab, 0x39, 0xe9]
        );
    }

    #[test]
    fn variant_matches_the_windows_target_abi_size() {
        let expected = if cfg!(target_pointer_width = "64") {
            24
        } else {
            16
        };
        assert_eq!(core::mem::size_of::<Variant>(), expected);
    }
}
