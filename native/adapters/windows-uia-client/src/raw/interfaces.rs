//! COM interface layouts and vtables used by the fixed UI Automation client.

use core::ffi::c_void;

use super::{Guid, Hresult, Point, Rect, Variant};

#[repr(C)]
pub(crate) struct Unknown {
    pub(crate) vtable: *const UnknownVtable,
}

#[repr(C)]
pub(crate) struct UnknownVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: unsafe extern "system" fn(*mut Unknown) -> u32,
}

#[repr(C)]
pub(crate) struct Automation {
    pub(crate) vtable: *const AutomationVtable,
}

/// The prefix through `RemoveFocusChangedEventHandler` of `IUIAutomationVtbl`.
///
/// Slots not used by this host-only adapter are retained as pointer-sized
/// placeholders to preserve the Windows SDK vtable offset of the next slot.
#[repr(C)]
pub(crate) struct AutomationVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) compare_elements: *const c_void,
    pub(crate) compare_runtime_ids: *const c_void,
    pub(crate) get_root_element: *const c_void,
    pub(crate) element_from_handle:
        unsafe extern "system" fn(*mut Automation, isize, *mut *mut Element) -> Hresult,
    pub(crate) element_from_point:
        unsafe extern "system" fn(*mut Automation, Point, *mut *mut Element) -> Hresult,
    pub(crate) get_focused_element:
        unsafe extern "system" fn(*mut Automation, *mut *mut Element) -> Hresult,
    pub(crate) get_root_element_build_cache: *const c_void,
    pub(crate) element_from_handle_build_cache: *const c_void,
    pub(crate) element_from_point_build_cache: *const c_void,
    pub(crate) get_focused_element_build_cache: *const c_void,
    pub(crate) create_tree_walker: *const c_void,
    pub(crate) control_view_walker:
        unsafe extern "system" fn(*mut Automation, *mut *mut TreeWalker) -> Hresult,
    pub(crate) content_view_walker: *const c_void,
    pub(crate) raw_view_walker:
        unsafe extern "system" fn(*mut Automation, *mut *mut TreeWalker) -> Hresult,
    pub(crate) raw_view_condition: *const c_void,
    pub(crate) control_view_condition: *const c_void,
    pub(crate) content_view_condition: *const c_void,
    pub(crate) create_cache_request: *const c_void,
    pub(crate) create_true_condition: *const c_void,
    pub(crate) create_false_condition: *const c_void,
    pub(crate) create_property_condition: *const c_void,
    pub(crate) create_property_condition_ex: *const c_void,
    pub(crate) create_and_condition: *const c_void,
    pub(crate) create_and_condition_from_array: *const c_void,
    pub(crate) create_and_condition_from_native_array: *const c_void,
    pub(crate) create_or_condition: *const c_void,
    pub(crate) create_or_condition_from_array: *const c_void,
    pub(crate) create_or_condition_from_native_array: *const c_void,
    pub(crate) create_not_condition: *const c_void,
    pub(crate) add_automation_event_handler: unsafe extern "system" fn(
        *mut Automation,
        i32,
        *mut Element,
        i32,
        *mut c_void,
        *mut c_void,
    ) -> Hresult,
    pub(crate) remove_automation_event_handler:
        unsafe extern "system" fn(*mut Automation, i32, *mut Element, *mut c_void) -> Hresult,
    pub(crate) add_property_changed_event_handler_native_array: *const c_void,
    pub(crate) add_property_changed_event_handler: *const c_void,
    pub(crate) remove_property_changed_event_handler: *const c_void,
    pub(crate) add_structure_changed_event_handler: unsafe extern "system" fn(
        *mut Automation,
        *mut Element,
        i32,
        *mut c_void,
        *mut c_void,
    ) -> Hresult,
    pub(crate) remove_structure_changed_event_handler:
        unsafe extern "system" fn(*mut Automation, *mut Element, *mut c_void) -> Hresult,
    pub(crate) add_focus_changed_event_handler:
        unsafe extern "system" fn(*mut Automation, *mut c_void, *mut c_void) -> Hresult,
    pub(crate) remove_focus_changed_event_handler:
        unsafe extern "system" fn(*mut Automation, *mut c_void) -> Hresult,
}

#[repr(C)]
pub(crate) struct Element {
    pub(crate) vtable: *const ElementVtable,
}

/// The prefix through `get_CurrentBoundingRectangle` of `IUIAutomationElementVtbl`.
#[repr(C)]
pub(crate) struct ElementVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) set_focus: unsafe extern "system" fn(*mut Element) -> Hresult,
    pub(crate) get_runtime_id: *const c_void,
    pub(crate) find_first: *const c_void,
    pub(crate) find_all: *const c_void,
    pub(crate) find_first_build_cache: *const c_void,
    pub(crate) find_all_build_cache: *const c_void,
    pub(crate) build_updated_cache: *const c_void,
    pub(crate) current_property_value:
        unsafe extern "system" fn(*mut Element, i32, *mut Variant) -> Hresult,
    pub(crate) current_property_value_ex: *const c_void,
    pub(crate) cached_property_value: *const c_void,
    pub(crate) cached_property_value_ex: *const c_void,
    pub(crate) current_pattern_as:
        unsafe extern "system" fn(*mut Element, i32, *const Guid, *mut *mut c_void) -> Hresult,
    pub(crate) cached_pattern_as: *const c_void,
    pub(crate) current_pattern:
        unsafe extern "system" fn(*mut Element, i32, *mut *mut Unknown) -> Hresult,
    pub(crate) cached_pattern: *const c_void,
    pub(crate) cached_parent: *const c_void,
    pub(crate) cached_children: *const c_void,
    pub(crate) current_process_id: *const c_void,
    pub(crate) current_control_type: *const c_void,
    pub(crate) current_localized_control_type: *const c_void,
    pub(crate) current_name: *const c_void,
    pub(crate) current_accelerator_key: *const c_void,
    pub(crate) current_access_key: *const c_void,
    pub(crate) current_has_keyboard_focus: *const c_void,
    pub(crate) current_is_keyboard_focusable: *const c_void,
    pub(crate) current_is_enabled: *const c_void,
    pub(crate) current_automation_id: *const c_void,
    pub(crate) current_class_name: *const c_void,
    pub(crate) current_help_text: *const c_void,
    pub(crate) current_culture: *const c_void,
    pub(crate) current_is_control_element: *const c_void,
    pub(crate) current_is_content_element: *const c_void,
    pub(crate) current_is_password: *const c_void,
    pub(crate) current_native_window_handle: *const c_void,
    pub(crate) current_item_type: *const c_void,
    pub(crate) current_is_offscreen: *const c_void,
    pub(crate) current_orientation: *const c_void,
    pub(crate) current_framework_id: *const c_void,
    pub(crate) current_is_required_for_form: *const c_void,
    pub(crate) current_item_status: *const c_void,
    pub(crate) current_bounding_rectangle:
        unsafe extern "system" fn(*mut Element, *mut Rect) -> Hresult,
}

#[repr(C)]
pub(crate) struct ValuePattern {
    pub(crate) vtable: *const ValuePatternVtable,
}

#[repr(C)]
pub(crate) struct InvokePattern {
    pub(crate) vtable: *const InvokePatternVtable,
}

/// The complete `IUIAutomationInvokePatternVtbl`.
#[repr(C)]
pub(crate) struct InvokePatternVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) invoke: unsafe extern "system" fn(*mut InvokePattern) -> Hresult,
}

/// The prefix through cached read-only state of the client-side
/// `IUIAutomationValuePatternVtbl`.
#[repr(C)]
pub(crate) struct ValuePatternVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) set_value: *const c_void,
    pub(crate) current_value:
        unsafe extern "system" fn(*mut ValuePattern, *mut *mut u16) -> Hresult,
    pub(crate) current_is_read_only:
        unsafe extern "system" fn(*mut ValuePattern, *mut i32) -> Hresult,
    pub(crate) cached_value: *const c_void,
    pub(crate) cached_is_read_only: *const c_void,
}

#[repr(C)]
pub(crate) struct TreeWalker {
    pub(crate) vtable: *const TreeWalkerVtable,
}

/// The prefix through sibling navigation of `IUIAutomationTreeWalkerVtbl`.
#[repr(C)]
pub(crate) struct TreeWalkerVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) parent: *const c_void,
    pub(crate) first_child:
        unsafe extern "system" fn(*mut TreeWalker, *mut Element, *mut *mut Element) -> Hresult,
    pub(crate) last_child: *const c_void,
    pub(crate) next_sibling:
        unsafe extern "system" fn(*mut TreeWalker, *mut Element, *mut *mut Element) -> Hresult,
}
