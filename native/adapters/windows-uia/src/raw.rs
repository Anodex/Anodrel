//! Narrow direct bindings for the UI Automation provider surface.
//!
//! Only the entry points and value shapes the semantic provider needs appear
//! here. There is no client-side UI Automation binding: this host publishes
//! accessibility outward and never reads a tree back.

#![allow(non_snake_case)]

use std::ffi::c_void;

pub type Handle = isize;
pub type Hresult = i32;
pub type Lresult = isize;

pub const S_OK: Hresult = 0;
pub const E_POINTER: Hresult = -2_147_467_261;
pub const E_NOINTERFACE: Hresult = -2_147_467_262;
pub const E_FAIL: Hresult = -2_147_467_259;

/// `UiaRootObjectId`, the object Windows asks for on `WM_GETOBJECT`.
pub const UIA_ROOT_OBJECT_ID: isize = -25;

/// `ProviderOptions_ServerSideProvider`.
pub const PROVIDER_OPTIONS_SERVER_SIDE: i32 = 0x1;

/// `UIA_WindowControlTypeId`.
pub const CONTROL_TYPE_WINDOW: i32 = 50_032;

pub const VT_EMPTY: u16 = 0;
pub const VT_I4: u16 = 3;
pub const VT_BSTR: u16 = 8;
pub const VT_BOOL: u16 = 11;

/// `VARIANT_TRUE`. A COM boolean is all-bits-set, not one.
pub const VARIANT_TRUE: i16 = -1;
pub const VARIANT_FALSE: i16 = 0;

/// A COM interface identifier.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

/// `IID_IUnknown`.
pub const IID_IUNKNOWN: Guid = Guid {
    data1: 0x0000_0000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

/// `IID_IRawElementProviderSimple`.
pub const IID_IRAW_ELEMENT_PROVIDER_SIMPLE: Guid = Guid {
    data1: 0xD6DD_68D1,
    data2: 0x86FD,
    data3: 0x4332,
    data4: [0x86, 0x66, 0x9A, 0xBE, 0xDE, 0xA2, 0xD2, 0x4C],
};

/// A COM `VARIANT`.
///
/// The header is four 16-bit fields and the union occupies sixteen bytes on
/// 64-bit Windows, giving the documented 24-byte layout. The union is held as
/// opaque storage because this provider only ever writes the three simple
/// variants below.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Variant {
    pub vt: u16,
    reserved1: u16,
    reserved2: u16,
    reserved3: u16,
    value: [u64; 2],
}

impl Variant {
    /// An empty variant, which is how a provider says "I do not supply this".
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            vt: VT_EMPTY,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            value: [0; 2],
        }
    }

    /// A signed 32-bit variant.
    #[must_use]
    pub const fn int(value: i32) -> Self {
        let mut variant = Self::empty();
        variant.vt = VT_I4;
        variant.value[0] = value as u32 as u64;
        variant
    }

    /// A COM boolean variant.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        let mut variant = Self::empty();
        variant.vt = VT_BOOL;
        let raw = if value { VARIANT_TRUE } else { VARIANT_FALSE };
        variant.value[0] = raw as u16 as u64;
        variant
    }

    /// A string variant owning a freshly allocated `BSTR`.
    ///
    /// Ownership passes to the caller, which releases it through
    /// `VariantClear`. Returns an empty variant if the allocation fails, so a
    /// provider never reports a string it does not own.
    #[must_use]
    pub fn string(text: &[u16]) -> Self {
        let length = match u32::try_from(text.len()) {
            Ok(length) => length,
            Err(_) => return Self::empty(),
        };
        // SAFETY: text is a live slice of exactly `length` UTF-16 units, which
        // SysAllocStringLen copies into its own allocation.
        let allocated = unsafe { SysAllocStringLen(text.as_ptr(), length) };
        if allocated.is_null() {
            return Self::empty();
        }
        let mut variant = Self::empty();
        variant.vt = VT_BSTR;
        variant.value[0] = allocated as usize as u64;
        variant
    }

    /// Returns a boolean value for internal tests without exposing the VARIANT
    /// union layout to the provider's other modules.
    #[cfg(test)]
    pub(crate) const fn boolean_value(&self) -> Option<bool> {
        if self.vt != VT_BOOL {
            return None;
        }
        Some((self.value[0] as u16 as i16) == VARIANT_TRUE)
    }
}

#[link(name = "oleaut32")]
unsafe extern "system" {
    fn SysAllocStringLen(text: *const u16, length: u32) -> *mut u16;
}

#[link(name = "uiautomationcore")]
unsafe extern "system" {
    /// Supplies the default window provider this one sits alongside.
    pub fn UiaHostProviderFromHwnd(window: Handle, provider: *mut *mut c_void) -> Hresult;

    /// Hands a provider back to Windows in answer to `WM_GETOBJECT`.
    pub fn UiaReturnRawElementProvider(
        window: Handle,
        wparam: usize,
        lparam: isize,
        provider: *mut c_void,
    ) -> Lresult;

}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn GetWindowTextW(window: Handle, text: *mut u16, count: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::{
        E_FAIL, E_NOINTERFACE, E_POINTER, IID_IRAW_ELEMENT_PROVIDER_SIMPLE, IID_IUNKNOWN, S_OK,
        VARIANT_TRUE, VT_BOOL, VT_EMPTY, VT_I4, Variant,
    };

    #[test]
    fn a_variant_has_the_documented_size_and_alignment() {
        // Windows reads this by layout. A wrong size would corrupt the caller's
        // stack rather than fail cleanly.
        assert_eq!(size_of::<Variant>(), 24);
        assert_eq!(align_of::<Variant>(), 8);
    }

    #[test]
    fn simple_variants_carry_their_tag_and_value() {
        assert_eq!(Variant::empty().vt, VT_EMPTY);
        assert_eq!(Variant::int(50_032).vt, VT_I4);
        assert_eq!(Variant::boolean(true).vt, VT_BOOL);
        assert_eq!(Variant::boolean(true).boolean_value(), Some(true));
        assert_eq!(Variant::boolean(false).boolean_value(), Some(false));
        // A COM boolean is all-bits-set, not one; reporting 1 makes some
        // clients read the value as false.
        assert_eq!(VARIANT_TRUE, -1);
    }

    #[test]
    fn interface_identifiers_match_their_published_values() {
        assert_eq!(IID_IUNKNOWN.data1, 0);
        assert_eq!(IID_IUNKNOWN.data4[7], 0x46);
        assert_eq!(IID_IRAW_ELEMENT_PROVIDER_SIMPLE.data1, 0xD6DD_68D1);
        assert_eq!(IID_IRAW_ELEMENT_PROVIDER_SIMPLE.data3, 0x4332);
    }

    #[test]
    fn failure_codes_are_the_documented_negative_values() {
        // A COM failure is signalled by the sign bit, so a positive value here
        // would read to every client as success.
        assert_eq!(S_OK, 0);
        for failure in [E_POINTER, E_NOINTERFACE, E_FAIL] {
            assert!(failure < 0, "{failure:#x} would read as success");
        }
    }
}
