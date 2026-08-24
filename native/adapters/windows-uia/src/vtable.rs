//! COM vtable layouts and static dispatch tables for the UI Automation provider.

use super::*;

/// `IRawElementProviderSimple`.
#[repr(C)]
pub(super) struct SimpleVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) get_provider_options: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
    pub(super) get_pattern_provider:
        unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> Hresult,
    pub(super) get_property_value:
        unsafe extern "system" fn(*mut c_void, i32, *mut Variant) -> Hresult,
    pub(super) get_host_raw_element_provider:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IRawElementProviderFragment`.
#[repr(C)]
pub(super) struct FragmentVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) navigate: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> Hresult,
    pub(super) get_runtime_id: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    pub(super) get_bounding_rectangle:
        unsafe extern "system" fn(*mut c_void, *mut UiaRect) -> Hresult,
    pub(super) get_embedded_fragment_roots:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    pub(super) set_focus: unsafe extern "system" fn(*mut c_void) -> Hresult,
    pub(super) get_fragment_root:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IRawElementProviderFragmentRoot`.
#[repr(C)]
pub(super) struct FragmentRootVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) element_provider_from_point:
        unsafe extern "system" fn(*mut c_void, f64, f64, *mut *mut c_void) -> Hresult,
    pub(super) get_focus: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IInvokeProvider`.
#[repr(C)]
pub(super) struct InvokeVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) invoke: unsafe extern "system" fn(*mut c_void) -> Hresult,
}

/// `IValueProvider`.
#[repr(C)]
pub(super) struct ValueVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) set_value: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    pub(super) get_value: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> Hresult,
    pub(super) get_is_read_only: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
}

/// `IScrollProvider`.
#[repr(C)]
pub(super) struct ScrollVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) scroll: unsafe extern "system" fn(*mut c_void, i32, i32) -> Hresult,
    pub(super) set_scroll_percent: unsafe extern "system" fn(*mut c_void, f64, f64) -> Hresult,
    pub(super) get_horizontal_scroll_percent:
        unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    pub(super) get_vertical_scroll_percent:
        unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    pub(super) get_horizontal_view_size:
        unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    pub(super) get_vertical_view_size: unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    pub(super) get_horizontally_scrollable:
        unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
    pub(super) get_vertically_scrollable:
        unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
}

/// `IScrollItemProvider`.
#[repr(C)]
pub(super) struct ScrollItemVtbl {
    pub(super) query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    pub(super) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(super) scroll_into_view: unsafe extern "system" fn(*mut c_void) -> Hresult,
}

pub(super) static SIMPLE_VTBL: SimpleVtbl = SimpleVtbl {
    query_interface: simple_query_interface,
    add_ref: simple_add_ref,
    release: simple_release,
    get_provider_options,
    get_pattern_provider,
    get_property_value,
    get_host_raw_element_provider,
};

pub(super) static FRAGMENT_VTBL: FragmentVtbl = FragmentVtbl {
    query_interface: fragment_query_interface,
    add_ref: fragment_add_ref,
    release: fragment_release,
    navigate,
    get_runtime_id,
    get_bounding_rectangle,
    get_embedded_fragment_roots,
    set_focus,
    get_fragment_root,
};

pub(super) static FRAGMENT_ROOT_VTBL: FragmentRootVtbl = FragmentRootVtbl {
    query_interface: root_query_interface,
    add_ref: root_add_ref,
    release: root_release,
    element_provider_from_point,
    get_focus,
};

pub(super) static INVOKE_VTBL: InvokeVtbl = InvokeVtbl {
    query_interface: invoke_query_interface,
    add_ref: invoke_add_ref,
    release: invoke_release,
    invoke,
};

pub(super) static VALUE_VTBL: ValueVtbl = ValueVtbl {
    query_interface: value_query_interface,
    add_ref: value_add_ref,
    release: value_release,
    set_value,
    get_value,
    get_is_read_only,
};

pub(super) static SCROLL_VTBL: ScrollVtbl = ScrollVtbl {
    query_interface: scroll_query_interface,
    add_ref: scroll_add_ref,
    release: scroll_release,
    scroll,
    set_scroll_percent,
    get_horizontal_scroll_percent,
    get_vertical_scroll_percent,
    get_horizontal_view_size,
    get_vertical_view_size,
    get_horizontally_scrollable,
    get_vertically_scrollable,
};

pub(super) static SCROLL_ITEM_VTBL: ScrollItemVtbl = ScrollItemVtbl {
    query_interface: scroll_item_query_interface,
    add_ref: scroll_item_add_ref,
    release: scroll_item_release,
    scroll_into_view,
};
