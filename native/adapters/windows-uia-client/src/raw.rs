//! Exact ABI declarations for the narrow Windows UI Automation client slice.

#![allow(non_snake_case)]

use core::ffi::c_void;

pub(crate) type Hresult = i32;

pub(crate) const S_OK: Hresult = 0;
pub(crate) const COINIT_MULTITHREADED: u32 = 0;
pub(crate) const CLSCTX_INPROC_SERVER: u32 = 1;
pub(crate) const VT_I4: u16 = 3;
pub(crate) const VT_BSTR: u16 = 8;

pub(crate) const UIA_CONTROL_TYPE_PROPERTY_ID: i32 = 30_003;
pub(crate) const UIA_NAME_PROPERTY_ID: i32 = 30_005;
pub(crate) const UIA_AUTOMATION_ID_PROPERTY_ID: i32 = 30_011;
pub(crate) const UIA_VALUE_PATTERN_ID: i32 = 10_002;

/// The UI Automation client coclass from `UIAutomationClient.h`.
pub(crate) const CLSID_C_UI_AUTOMATION: Guid = Guid::new(
    0xff48_dba4,
    0x60ef,
    0x4201,
    [0xaa, 0x87, 0x54, 0x10, 0x3e, 0xef, 0x59, 0x4e],
);

/// The `IUIAutomation` interface from `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION: Guid = Guid::new(
    0x30cb_e57d,
    0xd9d0,
    0x452a,
    [0xab, 0x13, 0x7a, 0xc5, 0xac, 0x48, 0x25, 0xee],
);

/// The client-side `IUIAutomationValuePattern` interface from
/// `UIAutomationClient.h`.
pub(crate) const IID_I_UI_AUTOMATION_VALUE_PATTERN: Guid = Guid::new(
    0xa94c_d8b1,
    0x0844,
    0x4cd6,
    [0x9d, 0x2d, 0x64, 0x05, 0x37, 0xab, 0x39, 0xe9],
);

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

impl Guid {
    const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}

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

/// The prefix through `get_RawViewWalker` of `IUIAutomationVtbl`.
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
    pub(crate) current_pattern: *const c_void,
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
#[derive(Clone, Copy)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct Rect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
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

#[repr(C)]
pub(crate) union VariantValue {
    pub(crate) i4: i32,
    pub(crate) bstr: *mut u16,
    /// The documented VARIANT union holds two pointer-sized words. The property
    /// forms this client reads use only the first word, but this storage
    /// preserves the ABI size for every value Windows may write on 32- and
    /// 64-bit Windows.
    pub(crate) storage: [usize; 2],
}

#[repr(C)]
pub(crate) struct Variant {
    pub(crate) vt: u16,
    reserved1: u16,
    reserved2: u16,
    reserved3: u16,
    pub(crate) value: VariantValue,
}

impl Variant {
    pub(crate) const fn empty() -> Self {
        Self {
            vt: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            value: VariantValue { storage: [0; 2] },
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        // SAFETY: this is initialized VARIANT storage. `VariantClear` releases
        // only allocations declared by its current type tag and accepts an
        // empty variant unchanged.
        unsafe {
            let _ = VariantClear(self);
        }
    }
}

#[link(name = "ole32")]
unsafe extern "system" {
    pub(crate) fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> Hresult;
    pub(crate) fn CoUninitialize();
    pub(crate) fn CoCreateInstance(
        class: *const Guid,
        outer: *mut c_void,
        context: u32,
        interface: *const Guid,
        out: *mut *mut c_void,
    ) -> Hresult;
}

#[cfg(test)]
mod tests {
    use super::{
        AutomationVtable, ElementVtable, IID_I_UI_AUTOMATION_VALUE_PATTERN, Point, Rect,
        UIA_VALUE_PATTERN_ID, ValuePatternVtable, Variant,
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
            core::mem::size_of::<ValuePatternVtable>(),
            8 * core::mem::size_of::<*const core::ffi::c_void>()
        );
        assert_eq!(UIA_VALUE_PATTERN_ID, 10_002);
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

#[link(name = "oleaut32")]
unsafe extern "system" {
    pub(crate) fn VariantClear(value: *mut Variant) -> Hresult;
    pub(crate) fn SysStringLen(value: *const u16) -> u32;
    pub(crate) fn SysFreeString(value: *mut u16);
}
