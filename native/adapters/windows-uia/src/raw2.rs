//! Fragment-level UI Automation bindings and value shapes.

#![allow(non_snake_case)]

use std::ffi::c_void;

use crate::raw::{Guid, Hresult};

/// `IID_IRawElementProviderFragment`.
pub const IID_IRAW_ELEMENT_PROVIDER_FRAGMENT: Guid = Guid {
    data1: 0xF706_3DA8,
    data2: 0x8359,
    data3: 0x439C,
    data4: [0x92, 0x97, 0xBB, 0xC5, 0x29, 0x9A, 0x7D, 0x87],
};

/// `IID_IRawElementProviderFragmentRoot`.
pub const IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT: Guid = Guid {
    data1: 0x620C_E2A5,
    data2: 0xAB8F,
    data3: 0x40A9,
    data4: [0x86, 0xCB, 0xDE, 0x3C, 0x75, 0x59, 0x9B, 0x58],
};

/// `UIA_E_NOTSUPPORTED`.
///
/// The honest answer from a read-only provider asked to do something.
pub const UIA_E_NOTSUPPORTED: Hresult = -2_147_220_992;

/// `NavigateDirection` values.
pub mod direction {
    pub const PARENT: i32 = 0;
    pub const NEXT_SIBLING: i32 = 1;
    pub const PREVIOUS_SIBLING: i32 = 2;
    pub const FIRST_CHILD: i32 = 3;
    pub const LAST_CHILD: i32 = 4;
}

/// `VT_I4`, the element type of a runtime identifier array.
const VT_I4: u16 = 3;

/// A `UiaRect`: position and size in physical screen pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiaRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[link(name = "oleaut32")]
unsafe extern "system" {
    fn SafeArrayCreateVector(element_type: u16, lower_bound: i32, count: u32) -> *mut c_void;
    fn SafeArrayPutElement(
        array: *mut c_void,
        indices: *const i32,
        value: *const c_void,
    ) -> Hresult;
    fn SafeArrayDestroy(array: *mut c_void) -> Hresult;
}

/// Builds a `SAFEARRAY` of 32-bit integers for a runtime identifier.
///
/// Returns null if the array cannot be created or filled. A partially filled
/// array is destroyed rather than handed over, because a caller cannot tell a
/// short identifier from a wrong one.
#[must_use]
pub fn runtime_id_array(values: &[i32]) -> *mut c_void {
    let count = match u32::try_from(values.len()) {
        Ok(count) => count,
        Err(_) => return std::ptr::null_mut(),
    };
    // SAFETY: a vector safe array of VT_I4 with a zero lower bound is the
    // documented shape for a runtime identifier.
    let array = unsafe { SafeArrayCreateVector(VT_I4, 0, count) };
    if array.is_null() {
        return array;
    }
    for (index, value) in values.iter().enumerate() {
        let position = index as i32;
        // SAFETY: `array` has exactly `count` elements and `position` indexes
        // inside it; `value` is one readable i32 the call copies.
        let result = unsafe {
            SafeArrayPutElement(array, &position, std::ptr::from_ref(value).cast::<c_void>())
        };
        if result < 0 {
            // SAFETY: destroying the array this function alone owns.
            unsafe { SafeArrayDestroy(array) };
            return std::ptr::null_mut();
        }
    }
    array
}

#[cfg(test)]
mod tests {
    use super::{
        IID_IRAW_ELEMENT_PROVIDER_FRAGMENT, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
        UIA_E_NOTSUPPORTED, UiaRect, direction, runtime_id_array,
    };

    #[test]
    fn a_rect_has_the_documented_layout() {
        assert_eq!(size_of::<UiaRect>(), 32);
        assert_eq!(align_of::<UiaRect>(), 8);
    }

    #[test]
    fn fragment_identifiers_match_their_published_values() {
        assert_eq!(IID_IRAW_ELEMENT_PROVIDER_FRAGMENT.data1, 0xF706_3DA8);
        assert_eq!(IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT.data1, 0x620C_E2A5);
        assert_ne!(
            IID_IRAW_ELEMENT_PROVIDER_FRAGMENT,
            IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT
        );
    }

    #[test]
    fn navigation_directions_match_their_published_order() {
        assert_eq!(direction::PARENT, 0);
        assert_eq!(direction::NEXT_SIBLING, 1);
        assert_eq!(direction::PREVIOUS_SIBLING, 2);
        assert_eq!(direction::FIRST_CHILD, 3);
        assert_eq!(direction::LAST_CHILD, 4);
    }

    #[test]
    fn a_read_only_refusal_is_a_failure_code() {
        // A positive value would read to every client as success, which would
        // have a screen reader believe focus had moved.
        assert_eq!(UIA_E_NOTSUPPORTED >> 31, -1);
    }

    #[test]
    fn a_runtime_identifier_array_is_created_and_owned() {
        let array = runtime_id_array(&[3, 7]);
        assert!(!array.is_null());
        // SAFETY: destroying the array this test alone owns.
        unsafe { super::SafeArrayDestroy(array) };
    }
}
