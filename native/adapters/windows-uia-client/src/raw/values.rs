//! Windows value layouts with their associated ownership cleanup.

use super::functions::VariantClear;

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
pub(crate) union VariantValue {
    pub(crate) i4: i32,
    pub(crate) bool_value: i16,
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
