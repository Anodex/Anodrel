//! Raw property and tree-walker helpers for the fixed automation client.

use super::{UiAutomationElement, UiAutomationError};
use crate::{
    com::{Com, succeeded},
    raw,
};

/// Reads one BSTR property while Windows temporarily lends an element pointer.
///
/// The focus-event callback uses this for its sender while Windows guarantees
/// that sender remains valid. It copies the text before the callback returns,
/// so no UI Automation interface crosses from the callback thread to the
/// waiting diagnostic worker.
pub(super) fn text_property_from_raw(
    element: *mut raw::Element,
    property: i32,
) -> Result<String, UiAutomationError> {
    if element.is_null() {
        return Err(UiAutomationError::NullInterface);
    }
    let mut value = raw::Variant::empty();
    // SAFETY: this helper is called only for a live owned element or a sender
    // Windows temporarily lends to its callback. `value` is initialized
    // writable VARIANT storage for the documented property result.
    let result = unsafe {
        let vtable = (*element).vtable;
        ((*vtable).current_property_value)(element, property, &mut value)
    };
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    if value.vt != raw::VT_BSTR {
        return Err(UiAutomationError::PropertyType);
    }
    // SAFETY: a `VT_BSTR` variant carries exactly one BSTR pointer, which may
    // be null for an empty value. The Variant releases it on Drop.
    let text = unsafe { value.value.bstr };
    if text.is_null() {
        return Ok(String::new());
    }
    // SAFETY: SysStringLen reads the BSTR allocation Windows returned.
    let length = unsafe { raw::SysStringLen(text) } as usize;
    // SAFETY: the BSTR has `length` UTF-16 code units and remains owned by
    // `value` until this function returns.
    let units = unsafe { core::slice::from_raw_parts(text, length) };
    Ok(String::from_utf16(units)?)
}

#[derive(Clone, Copy)]
pub(super) enum TreeView {
    Raw,
    Control,
}

pub(super) fn walker_from(
    automation: &Com<raw::Automation>,
    view: TreeView,
) -> Result<Com<raw::TreeWalker>, UiAutomationError> {
    let mut walker = core::ptr::null_mut();
    // SAFETY: `automation` is a live `IUIAutomation` object, the selected
    // vtable slot is documented by UIAutomationClient.h, and `walker` is
    // writable storage for the one returned interface pointer.
    let result = unsafe {
        let vtable = (*automation.as_ptr()).vtable;
        match view {
            TreeView::Raw => ((*vtable).raw_view_walker)(automation.as_ptr(), &mut walker),
            TreeView::Control => ((*vtable).control_view_walker)(automation.as_ptr(), &mut walker),
        }
    };
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    Com::from_out(walker)
}

pub(super) fn walker_child(
    walker: &Com<raw::TreeWalker>,
    parent: &UiAutomationElement,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let mut child = core::ptr::null_mut();
    // SAFETY: the walker and parent element are live COM objects, and `child`
    // is writable storage for an optional returned interface.
    let result = unsafe {
        let vtable = (*walker.as_ptr()).vtable;
        ((*vtable).first_child)(walker.as_ptr(), parent.raw.as_ptr(), &mut child)
    };
    optional_element(result, child)
}

pub(super) fn walker_next_sibling(
    walker: &Com<raw::TreeWalker>,
    element: &UiAutomationElement,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    let mut sibling = core::ptr::null_mut();
    // SAFETY: the walker and element are live COM objects, and `sibling` is
    // writable storage for an optional returned interface.
    let result = unsafe {
        let vtable = (*walker.as_ptr()).vtable;
        ((*vtable).next_sibling)(walker.as_ptr(), element.raw.as_ptr(), &mut sibling)
    };
    optional_element(result, sibling)
}

pub(super) fn optional_element(
    result: raw::Hresult,
    pointer: *mut raw::Element,
) -> Result<Option<UiAutomationElement>, UiAutomationError> {
    if !succeeded(result) {
        return Err(UiAutomationError::Query(result));
    }
    Ok(
        core::ptr::NonNull::new(pointer).map(|pointer| UiAutomationElement {
            raw: Com::from_out(pointer.as_ptr()).expect("non-null COM element remains non-null"),
        }),
    )
}
