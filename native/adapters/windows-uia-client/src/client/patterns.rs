//! Client-side Value and Invoke pattern access for fixed host diagnostics.
//!
//! These COM calls stay separate from tree and property traversal because they
//! have different ownership rules: a pattern interface is optional and the
//! Value getter transfers a `BSTR` that this module must release.

use std::ffi::c_void;

use crate::{
    com::{Com, succeeded},
    raw,
};

use super::{UiAutomationClient, UiAutomationElement, UiAutomationError};

/// One client-side Invoke interface prepared for a fixed host diagnostic.
///
/// It owns the interface Windows returned for a previously selected element
/// and consumes itself on the one standard Invoke call. The direct-client
/// adapter remains host-only: no Anodrel application, protocol message, or SDK
/// caller can create an element, receive this interface, or observe its result.
pub struct UiAutomationInvocation {
    pattern: Com<raw::InvokePattern>,
}

impl UiAutomationInvocation {
    /// Invokes the already-selected control exactly once.
    pub fn invoke(self) -> Result<(), UiAutomationError> {
        // SAFETY: this guard owns the exact client-side Invoke interface
        // Windows returned, and the vtable slot is the documented
        // `IUIAutomationInvokePattern::Invoke` member.
        let result = unsafe {
            let vtable = (*self.pattern.as_ptr()).vtable;
            ((*vtable).invoke)(self.pattern.as_ptr())
        };
        if succeeded(result) {
            Ok(())
        } else {
            Err(UiAutomationError::Query(result))
        }
    }
}

impl UiAutomationClient {
    pub(super) fn value_pattern(
        &self,
        element: &UiAutomationElement,
    ) -> Result<Option<Com<raw::ValuePattern>>, UiAutomationError> {
        let mut pattern: *mut c_void = core::ptr::null_mut();
        // SAFETY: the element is live, both identifiers are fixed Windows SDK
        // values, and `pattern` is writable output storage for the client-side
        // Value pattern interface.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).current_pattern_as)(
                element.raw.as_ptr(),
                raw::UIA_VALUE_PATTERN_ID,
                &raw::IID_I_UI_AUTOMATION_VALUE_PATTERN,
                &mut pattern,
            )
        };
        optional_pattern(result, pattern)
    }

    pub(super) fn invoke_pattern(
        &self,
        element: &UiAutomationElement,
    ) -> Result<Option<Com<raw::InvokePattern>>, UiAutomationError> {
        let mut pattern: *mut c_void = core::ptr::null_mut();
        // SAFETY: the element is live, both identifiers are fixed Windows SDK
        // values, and `pattern` is writable output storage for the client-side
        // Invoke pattern interface.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).current_pattern_as)(
                element.raw.as_ptr(),
                raw::UIA_INVOKE_PATTERN_ID,
                &raw::IID_I_UI_AUTOMATION_INVOKE_PATTERN,
                &mut pattern,
            )
        };
        optional_pattern(result, pattern)
    }

    /// Obtains the one standard Invoke interface for a fixed diagnostic node.
    ///
    /// Holding the returned interface lets a short-lived event diagnostic arm
    /// its private listener before it performs the one already-selected action.
    /// No target, input, result, or interface crosses into an application.
    pub fn prepare_invoke(
        &self,
        element: &UiAutomationElement,
    ) -> Result<UiAutomationInvocation, UiAutomationError> {
        let Some(pattern) = self.invoke_pattern(element)? else {
            return Err(UiAutomationError::UnexpectedTree);
        };
        Ok(UiAutomationInvocation { pattern })
    }
}

fn optional_pattern<T>(
    result: raw::Hresult,
    pattern: *mut c_void,
) -> Result<Option<Com<T>>, UiAutomationError> {
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    let Some(pattern) = core::ptr::NonNull::new(pattern) else {
        return Ok(None);
    };
    Ok(Some(Com::from_out(pattern.as_ptr().cast())?))
}

/// Releases the BSTR a Windows Value-pattern getter returns after decoding it.
struct OwnedBstr(*mut u16);

impl OwnedBstr {
    fn decode(&self) -> Result<String, UiAutomationError> {
        if self.0.is_null() {
            return Ok(String::new());
        }
        // SAFETY: the BSTR pointer comes from a successful Windows pattern
        // getter and remains owned by this guard through the conversion.
        let length = unsafe { raw::SysStringLen(self.0) } as usize;
        // SAFETY: a BSTR has exactly `length` UTF-16 code units.
        let units = unsafe { core::slice::from_raw_parts(self.0, length) };
        Ok(String::from_utf16(units)?)
    }
}

impl Drop for OwnedBstr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this guard exclusively owns the BSTR returned by the
            // successful Value-pattern getter.
            unsafe { raw::SysFreeString(self.0) };
        }
    }
}

pub(super) fn value_pattern_value(
    pattern: &Com<raw::ValuePattern>,
) -> Result<String, UiAutomationError> {
    let mut value = core::ptr::null_mut();
    // SAFETY: `pattern` owns a live client-side Value pattern and `value`
    // is writable BSTR output storage whose ownership transfers here.
    let result = unsafe {
        let vtable = (*pattern.as_ptr()).vtable;
        ((*vtable).current_value)(pattern.as_ptr(), &mut value)
    };
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    OwnedBstr(value).decode()
}

pub(super) fn value_pattern_is_read_only(
    pattern: &Com<raw::ValuePattern>,
) -> Result<bool, UiAutomationError> {
    let mut read_only = 0;
    // SAFETY: `pattern` owns a live client-side Value pattern and
    // `read_only` is writable output storage for its documented BOOL value.
    let result = unsafe {
        let vtable = (*pattern.as_ptr()).vtable;
        ((*vtable).current_is_read_only)(pattern.as_ptr(), &mut read_only)
    };
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    Ok(read_only != 0)
}
