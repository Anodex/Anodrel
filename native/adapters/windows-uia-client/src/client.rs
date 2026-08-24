//! Read-only element lookup, tree walking, and property decoding.

use std::{ffi::c_void, fmt, string::FromUtf16Error};

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

/// One read-only Value-pattern snapshot from a fixed host diagnostic field.
///
/// This data stays inside the host diagnostic adapter. It is not an
/// application protocol result and carries no field selector, write operation,
/// or live subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiAutomationValue {
    /// The field's copied UTF-16 value.
    pub value: String,
    /// Whether UI Automation may write the value.
    pub is_read_only: bool,
}

/// A UI Automation screen rectangle in physical pixels.
///
/// It is copied from Windows only inside the host diagnostic adapter. The
/// platform protocol and application SDK expose neither this geometry nor an
/// operation that can choose a coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiAutomationRect {
    /// Left edge in screen pixels.
    pub left: i32,
    /// Top edge in screen pixels.
    pub top: i32,
    /// Right edge in screen pixels.
    pub right: i32,
    /// Bottom edge in screen pixels.
    pub bottom: i32,
}

impl UiAutomationRect {
    /// Returns whether Windows reported no visible area for this rectangle.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// Returns whether another non-empty rectangle fits inside this one.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    fn center(self) -> Option<raw::Point> {
        if self.is_empty() {
            return None;
        }
        Some(raw::Point {
            x: ((i64::from(self.left) + i64::from(self.right)) / 2) as i32,
            y: ((i64::from(self.top) + i64::from(self.bottom)) / 2) as i32,
        })
    }
}

/// One owned immutable UI Automation element interface.
///
/// It deliberately exposes no raw pointer, COM operation, pattern, or mutable
/// state. Only [`UiAutomationClient`] can read its closed diagnostic values.
pub struct UiAutomationElement {
    raw: Com<raw::Element>,
}

/// A direct read-only UI Automation client and its raw/control tree walkers.
pub struct UiAutomationClient {
    automation: Com<raw::Automation>,
    raw_view_walker: Com<raw::TreeWalker>,
    control_view_walker: Com<raw::TreeWalker>,
}

impl UiAutomationClient {
    /// Creates the direct UI Automation client on an initialized COM thread.
    pub fn connect() -> Result<Self, UiAutomationError> {
        let automation = create_automation()?;
        let raw_view_walker = walker_from(&automation, TreeView::Raw)?;
        let control_view_walker = walker_from(&automation, TreeView::Control)?;
        Ok(Self {
            automation,
            raw_view_walker,
            control_view_walker,
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
        self.children_with(&self.raw_view_walker, parent)
    }

    /// Returns the element's direct control-view children in published order.
    ///
    /// The control view is the filtered tree Windows accessibility clients use
    /// to navigate controls. It remains host-diagnostic data; applications
    /// receive neither this tree nor a result derived from it.
    pub fn control_children(
        &self,
        parent: &UiAutomationElement,
    ) -> Result<Vec<UiAutomationElement>, UiAutomationError> {
        self.children_with(&self.control_view_walker, parent)
    }

    /// Returns an element's current screen rectangle through Windows.
    pub fn bounding_rectangle(
        &self,
        element: &UiAutomationElement,
    ) -> Result<UiAutomationRect, UiAutomationError> {
        let mut rectangle = raw::Rect::default();
        // SAFETY: the element is live and `rectangle` is writable storage for
        // the documented `get_CurrentBoundingRectangle` result.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).current_bounding_rectangle)(element.raw.as_ptr(), &mut rectangle)
        };
        if !succeeded(result) {
            return Err(UiAutomationError::Query(result));
        }
        Ok(UiAutomationRect {
            left: rectangle.left,
            top: rectangle.top,
            right: rectangle.right,
            bottom: rectangle.bottom,
        })
    }

    /// Resolves the element at the centre of one already-published rectangle.
    ///
    /// This deliberately cannot accept an arbitrary point. It is only the
    /// fixed geometry verification primitive used by the host diagnostic.
    pub fn element_at_center(
        &self,
        element: &UiAutomationElement,
    ) -> Result<Option<UiAutomationElement>, UiAutomationError> {
        let Some(point) = self.bounding_rectangle(element)?.center() else {
            return Ok(None);
        };
        let mut hit = core::ptr::null_mut();
        // SAFETY: `automation` is live, `point` was derived from the selected
        // element's current rectangle, and `hit` is writable output storage.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).element_from_point)(self.automation.as_ptr(), point, &mut hit)
        };
        optional_element(result, hit)
    }

    /// Reads the one standard Value pattern allowed by this diagnostic adapter.
    ///
    /// A missing pattern is normal for non-Edit elements. This method never
    /// calls the pattern's `SetValue` member and accepts no value to write.
    pub fn read_value_pattern(
        &self,
        element: &UiAutomationElement,
    ) -> Result<Option<UiAutomationValue>, UiAutomationError> {
        let Some(pattern) = self.value_pattern(element)? else {
            return Ok(None);
        };
        Ok(Some(UiAutomationValue {
            value: value_pattern_value(&pattern)?,
            is_read_only: value_pattern_is_read_only(&pattern)?,
        }))
    }

    fn children_with(
        &self,
        walker: &Com<raw::TreeWalker>,
        parent: &UiAutomationElement,
    ) -> Result<Vec<UiAutomationElement>, UiAutomationError> {
        let mut current = walker_child(walker, parent)?;
        let mut children = Vec::new();
        while let Some(element) = current {
            current = walker_next_sibling(walker, &element)?;
            children.push(element);
        }
        Ok(children)
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

    fn value_pattern(
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
        if !succeeded(result) {
            return Err(UiAutomationError::Query(result));
        }
        let Some(pattern) = core::ptr::NonNull::new(pattern) else {
            return Ok(None);
        };
        Ok(Some(Com::from_out(pattern.as_ptr().cast())?))
    }
}

/// Releases the BSTR a Windows pattern getter returns after decoding it.
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

fn value_pattern_value(pattern: &Com<raw::ValuePattern>) -> Result<String, UiAutomationError> {
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

fn value_pattern_is_read_only(pattern: &Com<raw::ValuePattern>) -> Result<bool, UiAutomationError> {
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

#[derive(Clone, Copy)]
enum TreeView {
    Raw,
    Control,
}

fn walker_from(
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

fn walker_child(
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

fn walker_next_sibling(
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

#[cfg(test)]
mod tests {
    use super::UiAutomationRect;

    #[test]
    fn rectangle_containment_rejects_empty_and_outside_rectangles() {
        let root = UiAutomationRect {
            left: -100,
            top: -50,
            right: 300,
            bottom: 250,
        };
        assert!(root.contains(UiAutomationRect {
            left: -100,
            top: -50,
            right: 300,
            bottom: 250,
        }));
        assert!(!root.contains(UiAutomationRect {
            left: 300,
            top: 0,
            right: 300,
            bottom: 20,
        }));
        assert!(!root.contains(UiAutomationRect {
            left: -101,
            top: 0,
            right: 20,
            bottom: 20,
        }));
    }

    #[test]
    fn rectangle_centre_uses_wide_intermediate_arithmetic() {
        let rectangle = UiAutomationRect {
            left: i32::MIN,
            top: i32::MIN,
            right: i32::MAX,
            bottom: i32::MAX,
        };
        let centre = rectangle.center().expect("spanning rectangle has a centre");
        assert_eq!(centre.x, 0);
        assert_eq!(centre.y, 0);
    }
}
