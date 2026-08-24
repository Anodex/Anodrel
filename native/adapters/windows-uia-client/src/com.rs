//! COM apartment and reference ownership for the direct client adapter.

use core::{ffi::c_void, ptr::NonNull};

use crate::{UiAutomationError, raw};

/// One MTA COM apartment owned by the calling thread.
///
/// A client must create this before it constructs `IUIAutomation`. Dropping it
/// balances only a successful `CoInitializeEx` call on the same thread.
pub struct ComApartment {
    initialized: bool,
}

impl ComApartment {
    /// Initializes the current thread for the client-side MTA use case.
    pub fn initialize_mta() -> Result<Self, UiAutomationError> {
        // SAFETY: the null reserved pointer and documented MTA flag are the
        // complete `CoInitializeEx` contract. The result decides whether Drop
        // later calls `CoUninitialize`.
        let result =
            unsafe { raw::CoInitializeEx(core::ptr::null_mut(), raw::COINIT_MULTITHREADED) };
        if succeeded(result) {
            Ok(Self { initialized: true })
        } else {
            Err(UiAutomationError::Apartment(result))
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: only a successful `CoInitializeEx` on this same thread
            // sets `initialized`.
            unsafe { raw::CoUninitialize() };
        }
    }
}

/// One owned COM interface pointer.
pub(crate) struct Com<T> {
    pointer: NonNull<T>,
}

impl<T> Com<T> {
    pub(crate) fn from_out(pointer: *mut T) -> Result<Self, UiAutomationError> {
        NonNull::new(pointer)
            .map(|pointer| Self { pointer })
            .ok_or(UiAutomationError::NullInterface)
    }

    pub(crate) const fn as_ptr(&self) -> *mut T {
        self.pointer.as_ptr()
    }
}

impl<T> Drop for Com<T> {
    fn drop(&mut self) {
        // Every COM interface starts with the three `IUnknown` slots. The
        // pointer remains valid until this final owned reference is released.
        let unknown = self.pointer.as_ptr().cast::<raw::Unknown>();
        // SAFETY: `unknown` is an owned valid COM interface pointer and its
        // vtable begins with the documented IUnknown release slot.
        unsafe {
            let vtable = (*unknown).vtable;
            ((*vtable).release)(unknown);
        }
    }
}

pub(crate) const fn succeeded(result: raw::Hresult) -> bool {
    result >= raw::S_OK
}

pub(crate) fn create_automation() -> Result<Com<raw::Automation>, UiAutomationError> {
    let mut raw_interface: *mut c_void = core::ptr::null_mut();
    // SAFETY: both GUID pointers identify documented Windows UI Automation
    // values, aggregation is absent, and `raw_interface` is writable storage.
    let result = unsafe {
        raw::CoCreateInstance(
            &raw::CLSID_C_UI_AUTOMATION,
            core::ptr::null_mut(),
            raw::CLSCTX_INPROC_SERVER,
            &raw::IID_I_UI_AUTOMATION,
            &mut raw_interface,
        )
    };
    if !succeeded(result) {
        return Err(UiAutomationError::Create(result));
    }
    Com::from_out(raw_interface.cast::<raw::Automation>())
}
