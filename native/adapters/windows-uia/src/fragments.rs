//! UI Automation fragment and fragment-root navigation callbacks.
//!
//! Navigation and hit testing operate solely on one immutable provider tree and
//! return providers only through the guarded COM emission helpers.

use super::*;

pub(super) unsafe extern "system" fn get_property_value(
    this: *mut c_void,
    requested: i32,
    out: *mut Variant,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // Every path writes a variant, so an unsupported property leaves a
        // readable empty one rather than an uninitialised slot.
        // SAFETY: checked above.
        unsafe { *out = Variant::empty() };
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the simple vtable field of a live provider.
        let provider = unsafe { simple_of(this) };
        // SAFETY: the caller holds a reference; fields are immutable.
        let (element, tree) = unsafe { ((*provider).element, &(*provider).tree) };

        let Some(value) = tree.property(element, requested) else {
            return S_OK;
        };
        // SAFETY: `out` was checked above.
        unsafe { *out = value };
        S_OK
    })
}

pub(super) unsafe extern "system" fn get_host_raw_element_provider(
    this: *mut c_void,
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
        // SAFETY: the caller holds a reference.
        if !unsafe { (*provider).is_root() } {
            // Only the window has a host provider; an element is ours alone.
            return S_OK;
        }
        // SAFETY: the window handle is live for this call.
        unsafe { raw::UiaHostProviderFromHwnd((*provider).window, out) }
    })
}

pub(super) unsafe extern "system" fn navigate(
    this: *mut c_void,
    towards: i32,
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
        // SAFETY: `this` points at the fragment vtable field of a live provider.
        let provider = unsafe { fragment_of(this) };
        // SAFETY: the caller holds a reference; fields are immutable.
        let (element, tree) = unsafe { ((*provider).element, &(*provider).tree) };

        let Some(target) = tree.step(element, towards) else {
            return S_OK;
        };
        // SAFETY: the provider is live and `out` was checked above.
        unsafe { emit(provider, target, out) }
    })
}

pub(super) unsafe extern "system" fn get_runtime_id(
    this: *mut c_void,
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
        // SAFETY: `this` points at the fragment vtable field of a live provider.
        let provider = unsafe { fragment_of(this) };
        // SAFETY: the caller holds a reference; fields are immutable.
        let (element, tree) = unsafe { ((*provider).element, &(*provider).tree) };

        // The window root has no runtime identifier of its own: Windows
        // supplies one from the host provider.
        let Some(id) = tree.runtime_id(element) else {
            return S_OK;
        };
        // SAFETY: `out` was checked above; ownership of the array passes on.
        unsafe { *out = raw2::runtime_id_array(&id) };
        S_OK
    })
}

pub(super) unsafe extern "system" fn get_bounding_rectangle(
    this: *mut c_void,
    out: *mut UiaRect,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // An empty rectangle is the documented "nothing to point at" answer.
        // SAFETY: checked above.
        unsafe {
            *out = UiaRect {
                left: 0.0,
                top: 0.0,
                width: 0.0,
                height: 0.0,
            }
        };
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the fragment vtable field of a live provider.
        let provider = unsafe { fragment_of(this) };
        // SAFETY: the caller holds a reference; fields are immutable.
        let (element, tree) = unsafe { ((*provider).element, &(*provider).tree) };
        if let Some(rect) = tree.bounds(element) {
            // SAFETY: `out` was checked above.
            unsafe { *out = rect };
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn get_embedded_fragment_roots(
    _this: *mut c_void,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // There is no embedded root: the whole surface is one fragment.
        // SAFETY: checked above.
        unsafe { *out = ptr::null_mut() };
        S_OK
    })
}

pub(super) unsafe extern "system" fn set_focus(this: *mut c_void) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        // SAFETY: `this` points at the Fragment vtable field of a live provider.
        let provider = unsafe { fragment_of(this) };
        // SAFETY: the caller holds a reference; the element and tree are valid.
        let (element, tree) = unsafe { ((*provider).element, &(*provider).tree) };
        let Some(element) = element else {
            // The window root is already focused by UI Automation before it
            // reaches an element's SetFocus implementation.
            return UIA_E_NOTSUPPORTED;
        };
        if !tree.supports_focus(element) {
            return UIA_E_NOTSUPPORTED;
        }
        if tree.focus(element) { S_OK } else { E_FAIL }
    })
}

pub(super) unsafe extern "system" fn get_fragment_root(
    this: *mut c_void,
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
        // SAFETY: `this` points at the fragment vtable field of a live provider.
        let provider = unsafe { fragment_of(this) };
        // SAFETY: the provider is live; the root shares the same tree.
        let (window, tree) = unsafe { ((*provider).window, Arc::clone(&(*provider).tree)) };
        let root = Provider::create(window, None, tree);
        // SAFETY: `out` was checked above.
        unsafe { *out = (&raw mut (*root).fragment_root).cast::<c_void>() };
        S_OK
    })
}

pub(super) unsafe extern "system" fn element_provider_from_point(
    this: *mut c_void,
    x: f64,
    y: f64,
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
        // SAFETY: `this` points at the root vtable field of a live provider.
        let provider = unsafe { root_of(this) };
        // SAFETY: the caller holds a reference; fields are immutable.
        let hit = unsafe { (*provider).tree.element_at(x, y) };
        let Some(hit) = hit else {
            return S_OK;
        };
        // SAFETY: the provider is live and `out` was checked above.
        unsafe { emit(provider, Some(hit), out) }
    })
}

pub(super) unsafe extern "system" fn get_focus(
    this: *mut c_void,
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
        // SAFETY: `this` points at the root vtable field of a live provider.
        let provider = unsafe { root_of(this) };
        // SAFETY: the caller holds a reference; the immutable tree contains
        // only the focus snapshot that was valid when it was published.
        let focused = unsafe { (*provider).tree.focused() };
        let Some(focused) = focused else {
            return S_OK;
        };
        // SAFETY: the provider is live and `out` was checked above.
        unsafe { emit(provider, Some(focused), out) }
    })
}

/// Reads a window's title, bounded, as UTF-16 without a terminator.
pub(crate) fn window_title(window: Handle) -> Vec<u16> {
    const MAX_TITLE_UNITS: usize = 256;
    let mut buffer = vec![0_u16; MAX_TITLE_UNITS];
    // SAFETY: buffer is writable for exactly the declared count.
    let written =
        unsafe { raw::GetWindowTextW(window, buffer.as_mut_ptr(), MAX_TITLE_UNITS as i32) };
    let written = usize::try_from(written).unwrap_or(0).min(MAX_TITLE_UNITS);
    buffer.truncate(written);
    buffer
}
