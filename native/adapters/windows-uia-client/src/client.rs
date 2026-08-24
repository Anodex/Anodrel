//! Read-only element lookup, tree walking, and property decoding.

use std::{fmt, string::FromUtf16Error};

use crate::{
    com::{Com, create_automation, succeeded},
    raw,
};

/// A safe failure category from the direct Windows UI Automation client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAutomationError {
    /// The worker thread could not initialize its COM apartment.
    Apartment(i32),
    /// Windows could not create the UI Automation client.
    Create(i32),
    /// A UI Automation query failed.
    Query(i32),
    /// Windows returned a successful query with no interface pointer.
    NullInterface,
    /// A published property had a different VARIANT representation than expected.
    PropertyType,
    /// Windows returned a non-UTF-16 BSTR for a textual property.
    PropertyText,
    /// A fixed host diagnostic observed a different tree than its contract.
    UnexpectedTree,
}

impl fmt::Display for UiAutomationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Apartment(_) => "UI Automation client apartment could not initialize",
            Self::Create(_) => "UI Automation client could not be created",
            Self::Query(_) => "UI Automation client query failed",
            Self::NullInterface => "UI Automation client received no interface",
            Self::PropertyType => "UI Automation property had an unexpected representation",
            Self::PropertyText => "UI Automation property text was invalid",
            Self::UnexpectedTree => {
                "UI Automation tree differed from the fixed diagnostic contract"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiAutomationError {}

impl From<FromUtf16Error> for UiAutomationError {
    fn from(_: FromUtf16Error) -> Self {
        Self::PropertyText
    }
}

/// The fixed properties a host diagnostic may read from one UI Automation node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAutomationNode {
    /// The published user-facing name, empty when the semantic role has none.
    pub name: String,
    /// The published semantic document identifier.
    pub automation_id: String,
    /// The published UI Automation control-type identifier.
    pub control_type: i32,
}

/// One owned immutable UI Automation element interface.
///
/// It deliberately exposes no raw pointer, COM operation, pattern, or mutable
/// state. Only [`UiAutomationClient`] can read its closed diagnostic values.
pub struct UiAutomationElement {
    raw: Com<raw::Element>,
}

/// A direct read-only UI Automation client and its raw-view tree walker.
pub struct UiAutomationClient {
    automation: Com<raw::Automation>,
    raw_view_walker: Com<raw::TreeWalker>,
}

impl UiAutomationClient {
    /// Creates the direct UI Automation client on an initialized COM thread.
    pub fn connect() -> Result<Self, UiAutomationError> {
        let automation = create_automation()?;
        let mut walker = core::ptr::null_mut();
        // SAFETY: `automation` is a live `IUIAutomation` object and `walker`
        // is writable storage for the one returned interface pointer.
        let result = unsafe {
            let vtable = (*automation.as_ptr()).vtable;
            ((*vtable).raw_view_walker)(automation.as_ptr(), &mut walker)
        };
        if !succeeded(result) {
            return Err(UiAutomationError::Query(result));
        }
        Ok(Self {
            automation,
            raw_view_walker: Com::from_out(walker)?,
        })
    }

    /// Obtains the published UI Automation root for one host-owned HWND.
    pub fn element_from_handle(
        &self,
        handle: isize,
    ) -> Result<UiAutomationElement, UiAutomationError> {
        let mut element = core::ptr::null_mut();
        // SAFETY: `automation` is live and `element` is writable storage for
        // the interface Windows returns for this opaque native handle.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).element_from_handle)(self.automation.as_ptr(), handle, &mut element)
        };
        if !succeeded(result) {
            return Err(UiAutomationError::Query(result));
        }
        Ok(UiAutomationElement {
            raw: Com::from_out(element)?,
        })
    }

    /// Reads the closed property set this diagnostic is allowed to inspect.
    pub fn node(
        &self,
        element: &UiAutomationElement,
    ) -> Result<UiAutomationNode, UiAutomationError> {
        Ok(UiAutomationNode {
            name: self.text_property(element, raw::UIA_NAME_PROPERTY_ID)?,
            automation_id: self.text_property(element, raw::UIA_AUTOMATION_ID_PROPERTY_ID)?,
            control_type: self.integer_property(element, raw::UIA_CONTROL_TYPE_PROPERTY_ID)?,
        })
    }

    /// Returns the element's direct raw-view children in published sibling order.
    pub fn raw_children(
        &self,
        parent: &UiAutomationElement,
    ) -> Result<Vec<UiAutomationElement>, UiAutomationError> {
        let mut current = self.walker_child(parent)?;
        let mut children = Vec::new();
        while let Some(element) = current {
            current = self.walker_next_sibling(&element)?;
            children.push(element);
        }
        Ok(children)
    }

    fn walker_child(
        &self,
        parent: &UiAutomationElement,
    ) -> Result<Option<UiAutomationElement>, UiAutomationError> {
        let mut child = core::ptr::null_mut();
        // SAFETY: the walker and parent element are live COM objects, and
        // `child` is writable storage for an optional returned interface.
        let result = unsafe {
            let vtable = (*self.raw_view_walker.as_ptr()).vtable;
            ((*vtable).first_child)(
                self.raw_view_walker.as_ptr(),
                parent.raw.as_ptr(),
                &mut child,
            )
        };
        optional_element(result, child)
    }

    fn walker_next_sibling(
        &self,
        element: &UiAutomationElement,
    ) -> Result<Option<UiAutomationElement>, UiAutomationError> {
        let mut sibling = core::ptr::null_mut();
        // SAFETY: the walker and element are live COM objects, and `sibling`
        // is writable storage for an optional returned interface.
        let result = unsafe {
            let vtable = (*self.raw_view_walker.as_ptr()).vtable;
            ((*vtable).next_sibling)(
                self.raw_view_walker.as_ptr(),
                element.raw.as_ptr(),
                &mut sibling,
            )
        };
        optional_element(result, sibling)
    }

    fn text_property(
        &self,
        element: &UiAutomationElement,
        property: i32,
    ) -> Result<String, UiAutomationError> {
        let variant = self.property(element, property)?;
        if variant.vt != raw::VT_BSTR {
            return Err(UiAutomationError::PropertyType);
        }
        // SAFETY: a `VT_BSTR` variant carries exactly one BSTR pointer, which
        // may be null for an empty value. The Variant releases it on Drop.
        let text = unsafe { variant.value.bstr };
        if text.is_null() {
            return Ok(String::new());
        }
        // SAFETY: SysStringLen reads the BSTR allocation Windows returned.
        let length = unsafe { raw::SysStringLen(text) } as usize;
        // SAFETY: the BSTR has `length` UTF-16 code units and remains owned by
        // `variant` until this function returns.
        let units = unsafe { core::slice::from_raw_parts(text, length) };
        Ok(String::from_utf16(units)?)
    }

    fn integer_property(
        &self,
        element: &UiAutomationElement,
        property: i32,
    ) -> Result<i32, UiAutomationError> {
        let variant = self.property(element, property)?;
        if variant.vt != raw::VT_I4 {
            return Err(UiAutomationError::PropertyType);
        }
        // SAFETY: a `VT_I4` variant carries exactly one i32.
        Ok(unsafe { variant.value.i4 })
    }

    fn property(
        &self,
        element: &UiAutomationElement,
        property: i32,
    ) -> Result<raw::Variant, UiAutomationError> {
        let mut value = raw::Variant::empty();
        // SAFETY: the element is live and `value` is initialized writable
        // VARIANT storage for the exact property value Windows returns.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).current_property_value)(element.raw.as_ptr(), property, &mut value)
        };
        if succeeded(result) {
            Ok(value)
        } else {
            Err(UiAutomationError::Query(result))
        }
    }
}

fn optional_element(
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
