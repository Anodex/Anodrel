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
/// Slots not used by this read-only adapter are retained as pointer-sized
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
    pub(crate) element_from_point: *const c_void,
    pub(crate) get_focused_element: *const c_void,
    pub(crate) get_root_element_build_cache: *const c_void,
    pub(crate) element_from_handle_build_cache: *const c_void,
    pub(crate) element_from_point_build_cache: *const c_void,
    pub(crate) get_focused_element_build_cache: *const c_void,
    pub(crate) create_tree_walker: *const c_void,
    pub(crate) control_view_walker: *const c_void,
    pub(crate) content_view_walker: *const c_void,
    pub(crate) raw_view_walker:
        unsafe extern "system" fn(*mut Automation, *mut *mut TreeWalker) -> Hresult,
}

#[repr(C)]
pub(crate) struct Element {
    pub(crate) vtable: *const ElementVtable,
}

/// The prefix through `GetCurrentPropertyValue` of `IUIAutomationElementVtbl`.
#[repr(C)]
pub(crate) struct ElementVtable {
    pub(crate) query_interface: *const c_void,
    pub(crate) add_ref: *const c_void,
    pub(crate) release: *const c_void,
    pub(crate) set_focus: *const c_void,
    pub(crate) get_runtime_id: *const c_void,
    pub(crate) find_first: *const c_void,
    pub(crate) find_all: *const c_void,
    pub(crate) find_first_build_cache: *const c_void,
    pub(crate) find_all_build_cache: *const c_void,
    pub(crate) build_updated_cache: *const c_void,
    pub(crate) current_property_value:
        unsafe extern "system" fn(*mut Element, i32, *mut Variant) -> Hresult,
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
    use super::Variant;

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
}
