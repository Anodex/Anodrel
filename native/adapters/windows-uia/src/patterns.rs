//! UI Automation pattern callbacks for Invoke, Value, Scroll, and ScrollItem.
//!
//! All callbacks resolve immutable provider state and route only through bounded
//! host mailboxes; no callback exposes a native handle or application callback.

use super::*;

pub(super) unsafe extern "system" fn get_provider_options(
    _this: *mut c_void,
    options: *mut i32,
) -> Hresult {
    contain(|| {
        if options.is_null() {
            return E_POINTER;
        }
        // SAFETY: checked above.
        unsafe { *options = raw::PROVIDER_OPTIONS_SERVER_SIDE };
        S_OK
    })
}

pub(super) unsafe extern "system" fn get_pattern_provider(
    this: *mut c_void,
    pattern: i32,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // SAFETY: checked above.
        unsafe { *out = ptr::null_mut() };
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the simple vtable field of a live provider.
        let provider = unsafe { simple_of(this) };
        // SAFETY: the caller holds a reference; immutable tree state makes the
        // pattern decision stable for this provider's published layout.
        let interface = unsafe {
            let provider_ref = &*provider;
            match provider_ref.element {
                Some(index)
                    if pattern == UIA_INVOKE_PATTERN_ID
                        && provider_ref.tree.supports_invoke(index) =>
                {
                    Some((&raw mut (*provider).invoke).cast::<c_void>())
                }
                Some(index)
                    if pattern == UIA_VALUE_PATTERN_ID
                        && provider_ref.tree.supports_value(index) =>
                {
                    Some((&raw mut (*provider).value).cast::<c_void>())
                }
                Some(index)
                    if pattern == UIA_SCROLL_PATTERN_ID
                        && provider_ref.tree.supports_scroll(index) =>
                {
                    Some((&raw mut (*provider).scroll).cast::<c_void>())
                }
                Some(index)
                    if pattern == UIA_SCROLL_ITEM_PATTERN_ID
                        && provider_ref.tree.supports_scroll_item(index) =>
                {
                    Some((&raw mut (*provider).scroll_item).cast::<c_void>())
                }
                _ => None,
            }
        };
        let Some(interface) = interface else {
            return S_OK;
        };
        // SAFETY: the provider is live and the new interface gains a reference.
        unsafe {
            increment(provider);
            *out = interface;
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn invoke(this: *mut c_void) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the Invoke vtable field of a live provider.
        let provider = unsafe { invoke_of(this) };
        // SAFETY: the caller holds a reference; both fields are immutable.
        let accepted = unsafe {
            (*provider)
                .element
                .is_some_and(|index| (*provider).tree.invoke(index))
        };
        if accepted { S_OK } else { E_FAIL }
    })
}

pub(super) unsafe extern "system" fn set_value(this: *mut c_void, _value: *const u16) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        // This deliberately does not read its supplied pointer. UI Automation
        // cannot turn into a second writer for host-owned field state.
        UIA_E_NOTSUPPORTED
    })
}

pub(super) unsafe extern "system" fn get_value(this: *mut c_void, out: *mut *mut u16) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // SAFETY: checked above.
        unsafe { *out = ptr::null_mut() };
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the Value vtable field of a live provider.
        let provider = unsafe { value_of(this) };
        // SAFETY: the caller holds a reference and the tree is immutable.
        let value = unsafe {
            (*provider)
                .element
                .and_then(|index| (*provider).tree.value(index))
        };
        let Some(value) = value else {
            return E_FAIL;
        };
        let Some(value) = raw::allocate_bstr(value) else {
            return E_FAIL;
        };
        // SAFETY: `out` is checked above and the BSTR ownership transfers to
        // the UI Automation client on success.
        unsafe { *out = value };
        S_OK
    })
}

pub(super) unsafe extern "system" fn get_is_read_only(this: *mut c_void, out: *mut i32) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // SAFETY: checked above.
        unsafe { *out = 0 };
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the Value vtable field of a live provider.
        let provider = unsafe { value_of(this) };
        // SAFETY: the caller holds a reference and the tree is immutable.
        let supported = unsafe {
            (*provider)
                .element
                .is_some_and(|index| (*provider).tree.supports_value(index))
        };
        if !supported {
            return E_FAIL;
        }
        // The field remains editable only through the host's normal local
        // input route. `SetValue` is unsupported, so this interface is
        // correctly read-only to automation.
        // SAFETY: `out` is checked above.
        unsafe { *out = 1 };
        S_OK
    })
}

pub(super) unsafe extern "system" fn scroll(
    this: *mut c_void,
    horizontal_amount: i32,
    vertical_amount: i32,
) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        if horizontal_amount != SCROLL_AMOUNT_NO_AMOUNT {
            return UIA_E_NOTSUPPORTED;
        }
        let command = match vertical_amount {
            SCROLL_AMOUNT_NO_AMOUNT => return S_OK,
            SCROLL_AMOUNT_SMALL_DECREMENT => UiAutomationScrollCommand::Line { forward: false },
            SCROLL_AMOUNT_SMALL_INCREMENT => UiAutomationScrollCommand::Line { forward: true },
            SCROLL_AMOUNT_LARGE_DECREMENT => UiAutomationScrollCommand::Page { forward: false },
            SCROLL_AMOUNT_LARGE_INCREMENT => UiAutomationScrollCommand::Page { forward: true },
            _ => return UIA_E_NOTSUPPORTED,
        };
        // SAFETY: this points at the Scroll vtable field of a live provider.
        let provider = unsafe { scroll_of(this) };
        // SAFETY: the caller holds a reference; the immutable tree checks that
        // this exact element is still the publication's selected scroll group.
        let accepted = unsafe {
            (*provider)
                .element
                .is_some_and(|index| (*provider).tree.scroll(index, command))
        };
        if accepted { S_OK } else { E_FAIL }
    })
}

pub(super) unsafe extern "system" fn set_scroll_percent(
    this: *mut c_void,
    horizontal_percent: f64,
    vertical_percent: f64,
) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        if horizontal_percent != UIA_SCROLL_PATTERN_NO_SCROLL
            || !vertical_percent.is_finite()
            || !(0.0..=100.0).contains(&vertical_percent)
        {
            return UIA_E_NOTSUPPORTED;
        }
        // SAFETY: this points at the Scroll vtable field of a live provider.
        let provider = unsafe { scroll_of(this) };
        // SAFETY: as above.
        let accepted = unsafe {
            (*provider).element.is_some_and(|index| {
                (*provider).tree.scroll(
                    index,
                    UiAutomationScrollCommand::Percent {
                        percent: vertical_percent,
                    },
                )
            })
        };
        if accepted { S_OK } else { E_FAIL }
    })
}

pub(super) unsafe extern "system" fn scroll_into_view(this: *mut c_void) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: this points at the ScrollItem vtable field of a live
        // provider. The immutable tree verifies that this exact element was
        // one of the host-selected viewport's permitted descendants.
        let provider = unsafe { scroll_item_of(this) };
        // SAFETY: the caller holds a reference and the route is the only path
        // back to the owning UI thread.
        let accepted = unsafe {
            (*provider)
                .element
                .is_some_and(|index| (*provider).tree.scroll_into_view(index))
        };
        if accepted { S_OK } else { E_FAIL }
    })
}

pub(super) unsafe extern "system" fn get_horizontal_scroll_percent(
    this: *mut c_void,
    out: *mut f64,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe { get_scroll_double(this, out, |_| UIA_SCROLL_PATTERN_NO_SCROLL) }
}

pub(super) unsafe extern "system" fn get_vertical_scroll_percent(
    this: *mut c_void,
    out: *mut f64,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe {
        get_scroll_double(
            this,
            out,
            UiAutomationScrollSnapshot::vertical_scroll_percent,
        )
    }
}

pub(super) unsafe extern "system" fn get_horizontal_view_size(
    this: *mut c_void,
    out: *mut f64,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe { get_scroll_double(this, out, |_| 100.0) }
}

pub(super) unsafe extern "system" fn get_vertical_view_size(
    this: *mut c_void,
    out: *mut f64,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe { get_scroll_double(this, out, UiAutomationScrollSnapshot::vertical_view_size) }
}

pub(super) unsafe extern "system" fn get_horizontally_scrollable(
    this: *mut c_void,
    out: *mut i32,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe { get_scroll_bool(this, out, false) }
}

pub(super) unsafe extern "system" fn get_vertically_scrollable(
    this: *mut c_void,
    out: *mut i32,
) -> Hresult {
    // SAFETY: the COM caller supplied this interface pointer.
    unsafe { get_scroll_bool(this, out, true) }
}

pub(super) unsafe fn get_scroll_double(
    this: *mut c_void,
    out: *mut f64,
    value: impl FnOnce(&UiAutomationScrollSnapshot) -> f64,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: this points at the Scroll vtable field of a live provider.
        let snapshot = unsafe { scroll_snapshot(this) };
        let Some(snapshot) = snapshot else {
            return E_FAIL;
        };
        // SAFETY: out was checked above.
        unsafe { *out = value(&snapshot) };
        S_OK
    })
}

pub(super) unsafe fn get_scroll_bool(this: *mut c_void, out: *mut i32, value: bool) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: this points at the Scroll vtable field of a live provider.
        if unsafe { scroll_snapshot(this) }.is_none() {
            return E_FAIL;
        }
        // SAFETY: out was checked above.
        unsafe { *out = i32::from(value) };
        S_OK
    })
}

/// Returns the immutable snapshot associated with one live Scroll interface.
///
/// # Safety
///
/// `this` must be the Scroll vtable pointer of a live provider.
pub(super) unsafe fn scroll_snapshot(this: *mut c_void) -> Option<UiAutomationScrollSnapshot> {
    // SAFETY: the caller supplies the matching live interface pointer.
    let provider = unsafe { scroll_of(this) };
    // SAFETY: the caller holds a COM reference; tree state is immutable.
    let index = unsafe { (*provider).element? };
    // SAFETY: as above.
    unsafe { (*provider).tree.scroll_snapshot(index).cloned() }
}
