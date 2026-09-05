//! Host-only element lookup, tree walking, property decoding, and fixed focus.

mod events;
mod geometry;
mod model;
mod patterns;
mod tree;

use std::{fmt, string::FromUtf16Error};

use crate::{
    com::{Com, create_automation, succeeded},
    raw,
};

pub use events::{
    UiAutomationFocusSubscription, UiAutomationLiveStatusSubscription,
    UiAutomationStructureSubscription,
};
pub use geometry::UiAutomationRect;
pub use model::{UiAutomationElement, UiAutomationNode, UiAutomationValue};
pub use patterns::UiAutomationInvocation;

use tree::{
    TreeView, optional_element, text_property_from_raw, walker_child, walker_from,
    walker_next_sibling,
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
    /// A fixed host diagnostic did not receive its bounded Windows event.
    EventNotObserved,
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
            Self::EventNotObserved => "UI Automation event was not delivered before its bound",
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

/// A host-only UI Automation client and its raw/control tree walkers.
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

    /// Asks Windows to focus one element selected by a host diagnostic.
    ///
    /// The element cannot be constructed from application data. This method
    /// exists only for the fixed UI Lab acceptance probe; it exposes no focus
    /// target, result, input, event, or control surface to the protocol or SDK.
    pub fn set_focus(&self, element: &UiAutomationElement) -> Result<(), UiAutomationError> {
        // SAFETY: the element is live and the exact vtable slot is the
        // documented `IUIAutomationElement::SetFocus` method.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).set_focus)(element.raw.as_ptr())
        };
        if succeeded(result) {
            Ok(())
        } else {
            Err(UiAutomationError::Query(result))
        }
    }

    /// Returns Windows' current focused element for the host focus diagnostic.
    ///
    /// The returned interface remains private to the diagnostic. No caller can
    /// select what is read or receive an identity, focus state, or subscription.
    pub fn focused_element(&self) -> Result<Option<UiAutomationElement>, UiAutomationError> {
        let mut element = core::ptr::null_mut();
        // SAFETY: `automation` is live and `element` is writable storage for
        // the optional focused element Windows returns.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).get_focused_element)(self.automation.as_ptr(), &mut element)
        };
        optional_element(result, element)
    }

    /// Returns whether a fixed host diagnostic element exposes `Invoke`.
    ///
    /// This inspects the standard pattern's presence only. It never obtains an
    /// Invoke-method interface, calls an action, or accepts an element from an
    /// application, so it cannot become an application interaction route.
    pub fn has_invoke_pattern(
        &self,
        element: &UiAutomationElement,
    ) -> Result<bool, UiAutomationError> {
        self.has_pattern(element, raw::UIA_INVOKE_PATTERN_ID)
    }

    /// Invokes one compiled action selected by a host acceptance diagnostic.
    ///
    /// The diagnostic derives the element from its own fixed authenticated
    /// session window. This client accepts neither an action ID nor input data,
    /// and exposes no Invoke result to an application, protocol, or SDK caller.
    pub fn invoke(&self, element: &UiAutomationElement) -> Result<(), UiAutomationError> {
        self.prepare_invoke(element)?.invoke()
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

    /// Reads whether Windows currently publishes this element as keyboard focused.
    ///
    /// The result is used only by the fixed host focus acceptance probe after
    /// it obtains a fresh provider publication. Applications cannot request a
    /// focus readback or receive this value through Anodrel.
    pub fn has_keyboard_focus(
        &self,
        element: &UiAutomationElement,
    ) -> Result<bool, UiAutomationError> {
        self.boolean_property(element, raw::UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID)
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
            value: patterns::value_pattern_value(&pattern)?,
            is_read_only: patterns::value_pattern_is_read_only(&pattern)?,
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
        text_property_from_raw(element.raw.as_ptr(), property)
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

    fn boolean_property(
        &self,
        element: &UiAutomationElement,
        property: i32,
    ) -> Result<bool, UiAutomationError> {
        let variant = self.property(element, property)?;
        if variant.vt != raw::VT_BOOL {
            return Err(UiAutomationError::PropertyType);
        }
        // SAFETY: a `VT_BOOL` variant carries exactly one Windows VARIANT_BOOL
        // value, where zero is false and negative one is true.
        match unsafe { variant.value.bool_value } {
            0 => Ok(false),
            -1 => Ok(true),
            _ => Err(UiAutomationError::PropertyType),
        }
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

    fn has_pattern(
        &self,
        element: &UiAutomationElement,
        pattern_id: i32,
    ) -> Result<bool, UiAutomationError> {
        let mut pattern = core::ptr::null_mut();
        // SAFETY: the element is live, `pattern_id` is a fixed Windows SDK
        // identifier chosen by this host diagnostic, and `pattern` is writable
        // storage for the optional IUnknown interface Windows returns.
        let result = unsafe {
            let vtable = (*element.raw.as_ptr()).vtable;
            ((*vtable).current_pattern)(element.raw.as_ptr(), pattern_id, &mut pattern)
        };
        if !succeeded(result) {
            return Err(UiAutomationError::Query(result));
        }
        let Some(pattern) = core::ptr::NonNull::new(pattern) else {
            return Ok(false);
        };
        let _pattern = Com::from_out(pattern.as_ptr())?;
        Ok(true)
    }
}
