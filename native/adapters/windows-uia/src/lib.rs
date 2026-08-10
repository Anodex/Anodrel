#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! The read-only UI Automation provider for one Anodrel window.
//!
//! This is the first of the staged provider slices described in
//! `docs/ACCESSIBILITY.md`: the window itself, answering
//! `IRawElementProviderSimple`. Semantic children are the next slice and are
//! not published yet.
//!
//! The provider is read-only and host-owned. It exposes no pattern, so nothing
//! can be invoked, toggled, scrolled, or edited through it, and it accepts
//! nothing from an application. Whether assistive technology is listening is
//! used only to avoid building a provider nobody asked for; that answer never
//! leaves this crate.

mod raw;

use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

use anodrel_windows_accessibility::property;
use raw::{
    CONTROL_TYPE_WINDOW, E_FAIL, E_NOINTERFACE, E_POINTER, Guid, Handle, Hresult,
    IID_IRAW_ELEMENT_PROVIDER_SIMPLE, IID_IUNKNOWN, Lresult, PROVIDER_OPTIONS_SERVER_SIDE, S_OK,
    UIA_ROOT_OBJECT_ID, Variant,
};

/// The fixed automation identifier for an Anodrel surface's root.
///
/// It is host-owned text. An application cannot supply or change it.
const ROOT_AUTOMATION_ID: &str = "anodrel.surface";

/// Longest window title this provider will report, in UTF-16 units.
const MAX_TITLE_UNITS: usize = 256;

/// Answers `WM_GETOBJECT` for one host window.
///
/// Returns `None` when the message is not asking for the UI Automation root, so
/// the caller can fall through to the default window procedure. Call this only
/// from the thread that owns `window`.
///
/// # Safety
///
/// `window` must be a live window owned by the calling thread.
#[must_use]
pub unsafe fn answer_get_object(window: Handle, wparam: usize, lparam: isize) -> Option<Lresult> {
    if lparam != UIA_ROOT_OBJECT_ID {
        return None;
    }
    // SAFETY: this only asks whether any client is listening and takes no
    // argument. Building a provider for nobody would be wasted work.
    if unsafe { raw::UiaClientsAreListening() } == 0 {
        return None;
    }

    let provider = RootProvider::create(window);
    // SAFETY: `provider` is a live COM object with one reference held here.
    // UiaReturnRawElementProvider takes its own reference before returning.
    let result = unsafe {
        raw::UiaReturnRawElementProvider(window, wparam, lparam, provider.cast::<c_void>())
    };
    // SAFETY: releasing the reference created above; Windows holds its own.
    unsafe { RootProvider::release_owned(provider) };
    Some(result)
}

/// The COM virtual table for `IRawElementProviderSimple`.
///
/// The layout is fixed by COM: the three `IUnknown` methods first, then the
/// interface's own four in declaration order.
#[repr(C)]
struct RootProviderVtbl {
    query_interface:
        unsafe extern "system" fn(*mut RootProvider, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut RootProvider) -> u32,
    release: unsafe extern "system" fn(*mut RootProvider) -> u32,
    get_provider_options: unsafe extern "system" fn(*mut RootProvider, *mut i32) -> Hresult,
    get_pattern_provider:
        unsafe extern "system" fn(*mut RootProvider, i32, *mut *mut c_void) -> Hresult,
    get_property_value: unsafe extern "system" fn(*mut RootProvider, i32, *mut Variant) -> Hresult,
    get_host_raw_element_provider:
        unsafe extern "system" fn(*mut RootProvider, *mut *mut c_void) -> Hresult,
}

static ROOT_PROVIDER_VTBL: RootProviderVtbl = RootProviderVtbl {
    query_interface,
    add_ref,
    release,
    get_provider_options,
    get_pattern_provider,
    get_property_value,
    get_host_raw_element_provider,
};

/// One reference-counted provider for a window's automation root.
///
/// Every field is set once at creation and never mutated, so the object is
/// safe to call from whichever thread UI Automation uses.
#[repr(C)]
struct RootProvider {
    vtable: *const RootProviderVtbl,
    references: AtomicU32,
    window: Handle,
    title: Vec<u16>,
}

impl RootProvider {
    /// Creates one provider holding a single reference.
    fn create(window: Handle) -> *mut Self {
        Box::into_raw(Box::new(Self {
            vtable: &raw const ROOT_PROVIDER_VTBL,
            references: AtomicU32::new(1),
            window,
            title: window_title(window),
        }))
    }

    /// Releases a reference this crate created.
    ///
    /// # Safety
    ///
    /// `provider` must be a pointer returned by [`Self::create`] whose
    /// reference has not already been released.
    unsafe fn release_owned(provider: *mut Self) {
        // SAFETY: the caller guarantees a live provider with a held reference.
        unsafe { release(provider) };
    }
}

/// Runs one COM method body, converting a panic into a failure code.
///
/// These are `extern "system"` and do not unwind, so an escaping panic would
/// abort the host — the same hazard the window procedure already contains.
fn contain(body: impl FnOnce() -> Hresult) -> Hresult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).unwrap_or(E_FAIL)
}

unsafe extern "system" fn query_interface(
    provider: *mut RootProvider,
    requested: *const Guid,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // SAFETY: `out` is a caller-provided output slot checked above.
        unsafe { *out = ptr::null_mut() };
        if provider.is_null() || requested.is_null() {
            return E_POINTER;
        }
        // SAFETY: COM guarantees `requested` points to one readable GUID.
        let requested = unsafe { *requested };
        if requested != IID_IUNKNOWN && requested != IID_IRAW_ELEMENT_PROVIDER_SIMPLE {
            return E_NOINTERFACE;
        }
        // SAFETY: the provider is live for the duration of this call.
        unsafe { add_ref(provider) };
        // SAFETY: `out` was checked above and receives the same pointer, which
        // is valid for both supported interfaces because the vtable begins
        // with IUnknown.
        unsafe { *out = provider.cast::<c_void>() };
        S_OK
    })
}

unsafe extern "system" fn add_ref(provider: *mut RootProvider) -> u32 {
    if provider.is_null() {
        return 0;
    }
    // SAFETY: the caller holds a reference, so the object is live.
    let references = unsafe { &(*provider).references };
    references.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn release(provider: *mut RootProvider) -> u32 {
    if provider.is_null() {
        return 0;
    }
    // SAFETY: the caller holds the reference being released.
    let previous = unsafe { &(*provider).references }.fetch_sub(1, Ordering::AcqRel);
    if previous != 1 {
        return previous - 1;
    }
    // SAFETY: this released the final reference, so ownership returns here and
    // the allocation is freed exactly once.
    drop(unsafe { Box::from_raw(provider) });
    0
}

unsafe extern "system" fn get_provider_options(
    _provider: *mut RootProvider,
    options: *mut i32,
) -> Hresult {
    contain(|| {
        if options.is_null() {
            return E_POINTER;
        }
        // SAFETY: `options` is a caller-provided output slot checked above.
        unsafe { *options = PROVIDER_OPTIONS_SERVER_SIDE };
        S_OK
    })
}

unsafe extern "system" fn get_pattern_provider(
    _provider: *mut RootProvider,
    _pattern: i32,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // No pattern is supported. This provider is read-only: nothing can be
        // invoked, toggled, scrolled, or edited through it.
        // SAFETY: `out` is a caller-provided output slot checked above.
        unsafe { *out = ptr::null_mut() };
        S_OK
    })
}

unsafe extern "system" fn get_property_value(
    provider: *mut RootProvider,
    requested: i32,
    out: *mut Variant,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // Every path writes a variant, so an unsupported property returns the
        // empty one rather than leaving the caller's slot uninitialised.
        // SAFETY: `out` is a caller-provided output slot checked above.
        unsafe { *out = Variant::empty() };
        if provider.is_null() {
            return E_POINTER;
        }
        // SAFETY: the caller holds a reference, so the object is live, and its
        // fields are immutable after creation.
        let title = unsafe { &(*provider).title };

        let value = match requested {
            property::NAME => Variant::string(title),
            property::CONTROL_TYPE => Variant::int(CONTROL_TYPE_WINDOW),
            property::AUTOMATION_ID => Variant::string(&utf16(ROOT_AUTOMATION_ID)),
            property::IS_CONTROL_ELEMENT | property::IS_CONTENT_ELEMENT => Variant::boolean(true),
            // The root is a container, not a target: focus belongs to the
            // semantic children the next slice publishes.
            property::IS_KEYBOARD_FOCUSABLE => Variant::boolean(false),
            property::IS_ENABLED => Variant::boolean(true),
            _ => return S_OK,
        };
        // SAFETY: `out` was checked above and is written exactly once more.
        unsafe { *out = value };
        S_OK
    })
}

unsafe extern "system" fn get_host_raw_element_provider(
    provider: *mut RootProvider,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // SAFETY: `out` is a caller-provided output slot checked above.
        unsafe { *out = ptr::null_mut() };
        if provider.is_null() {
            return E_POINTER;
        }
        // SAFETY: the caller holds a reference, so the window handle is live
        // for this call, and `out` receives the host provider Windows creates.
        unsafe { raw::UiaHostProviderFromHwnd((*provider).window, out) }
    })
}

/// Reads a window's title, bounded, as UTF-16 without a terminator.
fn window_title(window: Handle) -> Vec<u16> {
    let mut buffer = vec![0_u16; MAX_TITLE_UNITS];
    // SAFETY: buffer is writable for exactly its own length, which is what the
    // count argument declares.
    let written =
        unsafe { raw::GetWindowTextW(window, buffer.as_mut_ptr(), MAX_TITLE_UNITS as i32) };
    let written = usize::try_from(written).unwrap_or(0).min(MAX_TITLE_UNITS);
    buffer.truncate(written);
    buffer
}

/// Encodes host-owned text as UTF-16 without a terminator.
fn utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::c_void, ptr};

    use super::{
        CONTROL_TYPE_WINDOW, E_NOINTERFACE, E_POINTER, IID_IRAW_ELEMENT_PROVIDER_SIMPLE,
        IID_IUNKNOWN, PROVIDER_OPTIONS_SERVER_SIDE, ROOT_AUTOMATION_ID, RootProvider, S_OK,
        Variant, add_ref, contain, get_pattern_provider, get_property_value, get_provider_options,
        query_interface, raw, release, utf16,
    };
    use anodrel_windows_accessibility::property;

    /// Builds a provider for a window that does not exist.
    ///
    /// Every method under test reads only the object's own immutable fields, so
    /// no live window is required.
    fn provider() -> *mut RootProvider {
        RootProvider::create(0)
    }

    #[test]
    fn a_panicking_method_body_fails_instead_of_aborting() {
        // COM methods are `extern "system"` and do not unwind, so an escaping
        // panic would abort the host mid-call.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = contain(|| panic!("a provider defect must not abort the host"));
        std::panic::set_hook(previous);
        assert!(result < 0);
        assert_eq!(contain(|| S_OK), S_OK);
    }

    #[test]
    fn the_object_answers_only_the_interfaces_it_implements() {
        let provider = provider();
        let mut out: *mut c_void = ptr::null_mut();

        for supported in [IID_IUNKNOWN, IID_IRAW_ELEMENT_PROVIDER_SIMPLE] {
            // SAFETY: a live provider and valid output slot.
            assert_eq!(
                unsafe { query_interface(provider, &supported, &mut out) },
                S_OK
            );
            assert!(!out.is_null());
            // Each successful query took a reference, which is released here.
            // SAFETY: releasing exactly the reference the query added.
            unsafe { release(provider) };
        }

        let unsupported = super::Guid {
            data1: 1,
            data2: 2,
            data3: 3,
            data4: [4; 8],
        };
        // SAFETY: a live provider and valid output slot.
        assert_eq!(
            unsafe { query_interface(provider, &unsupported, &mut out) },
            E_NOINTERFACE
        );
        assert!(out.is_null(), "a refused query must clear its output");

        // SAFETY: releasing the original creation reference frees the object.
        unsafe { release(provider) };
    }

    #[test]
    fn every_method_rejects_a_null_output_rather_than_writing_through_it() {
        let provider = provider();
        // SAFETY: a live provider with deliberately null output slots.
        unsafe {
            assert_eq!(
                query_interface(provider, &IID_IUNKNOWN, ptr::null_mut()),
                E_POINTER
            );
            assert_eq!(get_provider_options(provider, ptr::null_mut()), E_POINTER);
            assert_eq!(
                get_pattern_provider(provider, 10_000, ptr::null_mut()),
                E_POINTER
            );
            assert_eq!(
                get_property_value(provider, property::NAME, ptr::null_mut()),
                E_POINTER
            );
            release(provider);
        }
    }

    #[test]
    fn reference_counting_frees_the_object_exactly_once() {
        let provider = provider();
        // SAFETY: a live provider held by this test.
        unsafe {
            assert_eq!(add_ref(provider), 2);
            assert_eq!(release(provider), 1);
            // The final release frees the allocation; the pointer is dead after.
            assert_eq!(release(provider), 0);
        }
    }

    #[test]
    fn the_provider_is_server_side_and_supports_no_pattern() {
        let provider = provider();
        let mut options = 0;
        let mut pattern: *mut c_void = ptr::null_mut();
        // SAFETY: a live provider and valid output slots.
        unsafe {
            assert_eq!(get_provider_options(provider, &mut options), S_OK);
            // Read-only: no pattern means nothing can be invoked through this.
            assert_eq!(get_pattern_provider(provider, 10_000, &mut pattern), S_OK);
            release(provider);
        }
        assert_eq!(options, PROVIDER_OPTIONS_SERVER_SIDE);
        assert!(pattern.is_null());
    }

    #[test]
    fn properties_report_the_documented_values_and_ignore_the_rest() {
        let provider = provider();
        let mut value = Variant::empty();
        // SAFETY: a live provider and a valid variant slot.
        unsafe {
            assert_eq!(
                get_property_value(provider, property::CONTROL_TYPE, &mut value),
                S_OK
            );
            assert_eq!(value.vt, raw::VT_I4);

            assert_eq!(
                get_property_value(provider, property::IS_CONTROL_ELEMENT, &mut value),
                S_OK
            );
            assert_eq!(value.vt, raw::VT_BOOL);

            // An unsupported property must still leave a readable variant.
            assert_eq!(get_property_value(provider, 30_006, &mut value), S_OK);
            assert_eq!(value.vt, raw::VT_EMPTY);

            release(provider);
        }
        assert_eq!(CONTROL_TYPE_WINDOW, 50_032);
    }

    #[test]
    fn the_root_automation_id_is_fixed_host_text() {
        // An application cannot supply or change it, so the root can never be
        // made to identify itself as something else.
        assert_eq!(ROOT_AUTOMATION_ID, "anodrel.surface");
        assert_eq!(utf16(ROOT_AUTOMATION_ID).len(), ROOT_AUTOMATION_ID.len());
    }
}
