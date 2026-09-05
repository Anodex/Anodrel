//! Direct Ole32 and OleAut32 entry points for the fixed client adapter.

#![allow(non_snake_case)]

use core::ffi::c_void;

use super::{Guid, Hresult, Variant};

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

#[link(name = "oleaut32")]
unsafe extern "system" {
    pub(crate) fn VariantClear(value: *mut Variant) -> Hresult;
    pub(crate) fn SysStringLen(value: *const u16) -> u32;
    pub(crate) fn SysFreeString(value: *mut u16);
}
