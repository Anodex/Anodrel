#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! The read-only UI Automation provider for one Anodrel window.
//!
//! The window answers as an automation root, and the semantic elements the
//! host mapped from its current layout answer as its children. See
//! `docs/ACCESSIBILITY.md`.
//!
//! The provider is read-only and host-owned. It supplies no pattern, so nothing
//! can be invoked, toggled, scrolled, or edited through it, and it refuses to
//! move focus. Nothing flows back to an application: it accepts no input, and
//! whether assistive technology is listening never leaves this crate.

mod raw;
mod raw2;
mod tree;

use std::{
    ffi::c_void,
    mem::offset_of,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use anodrel_windows_accessibility::AccessibleElement;
use raw::{
    E_FAIL, E_NOINTERFACE, E_POINTER, Guid, Handle, Hresult, IID_IRAW_ELEMENT_PROVIDER_SIMPLE,
    IID_IUNKNOWN, Lresult, S_OK, UIA_ROOT_OBJECT_ID, Variant,
};
use raw2::{
    IID_IRAW_ELEMENT_PROVIDER_FRAGMENT, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
    UIA_E_NOTSUPPORTED, UiaRect,
};
use tree::Tree;

pub use tree::publishable;

/// Answers `WM_GETOBJECT` for one host window.
///
/// `elements` are the semantic elements the host mapped from the window's
/// current layout; an empty list publishes a window with no children.
///
/// Returns `None` when the message is not asking for the UI Automation root, so
/// the caller falls through to the default window procedure.
///
/// # Safety
///
/// `window` must be a live window owned by the calling thread.
#[must_use]
pub unsafe fn answer_get_object(
    window: Handle,
    wparam: usize,
    lparam: isize,
    elements: Vec<AccessibleElement>,
) -> Option<Lresult> {
    if lparam != UIA_ROOT_OBJECT_ID {
        return None;
    }
    // SAFETY: this takes no argument and only reports whether a client exists.
    if unsafe { raw::UiaClientsAreListening() } == 0 {
        return None;
    }

    let tree = Arc::new(Tree::new(window_title(window), elements));
    let provider = Provider::create(window, None, tree);
    // SAFETY: `provider` is live with one reference held here, and
    // UiaReturnRawElementProvider takes its own before returning.
    let result = unsafe {
        raw::UiaReturnRawElementProvider(window, wparam, lparam, provider.cast::<c_void>())
    };
    // SAFETY: releasing the reference created above.
    unsafe { release_provider(provider) };
    Some(result)
}

/// `IRawElementProviderSimple`.
#[repr(C)]
struct SimpleVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_provider_options: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
    get_pattern_provider: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> Hresult,
    get_property_value: unsafe extern "system" fn(*mut c_void, i32, *mut Variant) -> Hresult,
    get_host_raw_element_provider:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IRawElementProviderFragment`.
#[repr(C)]
struct FragmentVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    navigate: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> Hresult,
    get_runtime_id: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    get_bounding_rectangle: unsafe extern "system" fn(*mut c_void, *mut UiaRect) -> Hresult,
    get_embedded_fragment_roots:
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    set_focus: unsafe extern "system" fn(*mut c_void) -> Hresult,
    get_fragment_root: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IRawElementProviderFragmentRoot`.
#[repr(C)]
struct FragmentRootVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    element_provider_from_point:
        unsafe extern "system" fn(*mut c_void, f64, f64, *mut *mut c_void) -> Hresult,
    get_focus: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

static SIMPLE_VTBL: SimpleVtbl = SimpleVtbl {
    query_interface: simple_query_interface,
    add_ref: simple_add_ref,
    release: simple_release,
    get_provider_options,
    get_pattern_provider,
    get_property_value,
    get_host_raw_element_provider,
};

static FRAGMENT_VTBL: FragmentVtbl = FragmentVtbl {
    query_interface: fragment_query_interface,
    add_ref: fragment_add_ref,
    release: fragment_release,
    navigate,
    get_runtime_id,
    get_bounding_rectangle,
    get_embedded_fragment_roots,
    set_focus,
    get_fragment_root,
};

static FRAGMENT_ROOT_VTBL: FragmentRootVtbl = FragmentRootVtbl {
    query_interface: root_query_interface,
    add_ref: root_add_ref,
    release: root_release,
    element_provider_from_point,
    get_focus,
};

/// One reference-counted provider for the window root or one of its elements.
///
/// COM reaches an object through the vtable pointer for the interface being
/// used, so all three sit at the front and each method recovers the object by
/// subtracting its own field offset. Every other field is set once at creation.
#[repr(C)]
struct Provider {
    simple: *const SimpleVtbl,
    fragment: *const FragmentVtbl,
    fragment_root: *const FragmentRootVtbl,
    references: AtomicU32,
    window: Handle,
    /// `None` for the window root, otherwise the element's position.
    element: Option<usize>,
    tree: Arc<Tree>,
}

impl Provider {
    fn create(window: Handle, element: Option<usize>, tree: Arc<Tree>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            simple: &raw const SIMPLE_VTBL,
            fragment: &raw const FRAGMENT_VTBL,
            fragment_root: &raw const FRAGMENT_ROOT_VTBL,
            references: AtomicU32::new(1),
            window,
            element,
            tree,
        }))
    }

    const fn is_root(&self) -> bool {
        self.element.is_none()
    }
}

/// Recovers the object from a pointer to one of its vtable fields.
///
/// # Safety
///
/// `this` must point to the field at `offset` of a live [`Provider`].
unsafe fn provider_from(this: *mut c_void, offset: usize) -> *mut Provider {
    unsafe { this.cast::<u8>().sub(offset).cast::<Provider>() }
}

unsafe fn simple_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, simple)) }
}

unsafe fn fragment_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, fragment)) }
}

unsafe fn root_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, fragment_root)) }
}

/// Runs one COM method body, converting a panic into a failure code.
///
/// These are `extern "system"` and do not unwind, so an escaping panic would
/// abort the host mid-call.
fn contain(body: impl FnOnce() -> Hresult) -> Hresult {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).unwrap_or(E_FAIL)
}

/// Answers `QueryInterface` for a live provider.
///
/// # Safety
///
/// `provider` must be live and `out` writable.
unsafe fn query(provider: *mut Provider, requested: *const Guid, out: *mut *mut c_void) -> Hresult {
    if out.is_null() {
        return E_POINTER;
    }
    // SAFETY: `out` is checked above.
    unsafe { *out = ptr::null_mut() };
    if provider.is_null() || requested.is_null() {
        return E_POINTER;
    }
    // SAFETY: COM guarantees one readable GUID.
    let requested = unsafe { *requested };
    // SAFETY: the caller holds a reference, so the object is live.
    let is_root = unsafe { (*provider).is_root() };

    let interface = if requested == IID_IUNKNOWN || requested == IID_IRAW_ELEMENT_PROVIDER_SIMPLE {
        // SAFETY: taking the address of a field of a live object.
        unsafe { (&raw mut (*provider).simple).cast::<c_void>() }
    } else if requested == IID_IRAW_ELEMENT_PROVIDER_FRAGMENT {
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).fragment).cast::<c_void>() }
    } else if requested == IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT && is_root {
        // Only the window is a fragment root; an element must not claim to be.
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).fragment_root).cast::<c_void>() }
    } else {
        return E_NOINTERFACE;
    };

    // SAFETY: the object is live and gains one reference for the new pointer.
    unsafe { increment(provider) };
    // SAFETY: `out` is checked above.
    unsafe { *out = interface };
    S_OK
}

/// # Safety
///
/// `provider` must be live.
unsafe fn increment(provider: *mut Provider) -> u32 {
    if provider.is_null() {
        return 0;
    }
    // SAFETY: the caller holds a reference.
    unsafe { &(*provider).references }.fetch_add(1, Ordering::Relaxed) + 1
}

/// # Safety
///
/// `provider` must be live and the caller must hold the reference released.
unsafe fn release_provider(provider: *mut Provider) -> u32 {
    if provider.is_null() {
        return 0;
    }
    // SAFETY: the caller holds the reference being released.
    let previous = unsafe { &(*provider).references }.fetch_sub(1, Ordering::AcqRel);
    if previous != 1 {
        return previous - 1;
    }
    // SAFETY: this released the final reference, so the allocation is freed
    // exactly once here.
    drop(unsafe { Box::from_raw(provider) });
    0
}

/// Creates a new provider for one element of the same tree and writes it out.
///
/// # Safety
///
/// `source` must be live and `out` writable.
unsafe fn emit(source: *mut Provider, element: Option<usize>, out: *mut *mut c_void) -> Hresult {
    // SAFETY: the caller holds a reference to `source`.
    let (window, tree) = unsafe { ((*source).window, Arc::clone(&(*source).tree)) };
    let created = Provider::create(window, element, tree);
    // SAFETY: `out` is checked by the caller; the fragment interface is what
    // navigation results are typed as.
    unsafe { *out = (&raw mut (*created).fragment).cast::<c_void>() };
    S_OK
}

macro_rules! forward_unknown {
    ($query:ident, $add:ident, $release:ident, $recover:ident) => {
        unsafe extern "system" fn $query(
            this: *mut c_void,
            requested: *const Guid,
            out: *mut *mut c_void,
        ) -> Hresult {
            contain(|| {
                if this.is_null() {
                    return E_POINTER;
                }
                // SAFETY: `this` points at the matching vtable field of a live
                // provider, which is how COM delivered it.
                unsafe { query($recover(this), requested, out) }
            })
        }

        unsafe extern "system" fn $add(this: *mut c_void) -> u32 {
            if this.is_null() {
                return 0;
            }
            // SAFETY: as above.
            unsafe { increment($recover(this)) }
        }

        unsafe extern "system" fn $release(this: *mut c_void) -> u32 {
            if this.is_null() {
                return 0;
            }
            // SAFETY: as above.
            unsafe { release_provider($recover(this)) }
        }
    };
}

forward_unknown!(
    simple_query_interface,
    simple_add_ref,
    simple_release,
    simple_of
);
forward_unknown!(
    fragment_query_interface,
    fragment_add_ref,
    fragment_release,
    fragment_of
);
forward_unknown!(root_query_interface, root_add_ref, root_release, root_of);

unsafe extern "system" fn get_provider_options(_this: *mut c_void, options: *mut i32) -> Hresult {
    contain(|| {
        if options.is_null() {
            return E_POINTER;
        }
        // SAFETY: checked above.
        unsafe { *options = raw::PROVIDER_OPTIONS_SERVER_SIDE };
        S_OK
    })
}

unsafe extern "system" fn get_pattern_provider(
    _this: *mut c_void,
    _pattern: i32,
    out: *mut *mut c_void,
) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // Read-only: no pattern is supplied, so nothing can be invoked,
        // toggled, scrolled, or edited through this provider.
        // SAFETY: checked above.
        unsafe { *out = ptr::null_mut() };
        S_OK
    })
}

unsafe extern "system" fn get_property_value(
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

unsafe extern "system" fn get_host_raw_element_provider(
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

unsafe extern "system" fn navigate(
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
        let (element, count) = unsafe { ((*provider).element, (*provider).tree.len()) };

        let Some(target) = tree::step(element, towards, count) else {
            return S_OK;
        };
        // SAFETY: the provider is live and `out` was checked above.
        unsafe { emit(provider, target, out) }
    })
}

unsafe extern "system" fn get_runtime_id(this: *mut c_void, out: *mut *mut c_void) -> Hresult {
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

unsafe extern "system" fn get_bounding_rectangle(this: *mut c_void, out: *mut UiaRect) -> Hresult {
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

unsafe extern "system" fn get_embedded_fragment_roots(
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

unsafe extern "system" fn set_focus(_this: *mut c_void) -> Hresult {
    // Read-only. Moving focus is an action, and this provider performs none.
    UIA_E_NOTSUPPORTED
}

unsafe extern "system" fn get_fragment_root(this: *mut c_void, out: *mut *mut c_void) -> Hresult {
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

unsafe extern "system" fn element_provider_from_point(
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

unsafe extern "system" fn get_focus(_this: *mut c_void, out: *mut *mut c_void) -> Hresult {
    contain(|| {
        if out.is_null() {
            return E_POINTER;
        }
        // Focus is not reported to assistive technology yet; that is its own
        // slice, and claiming a focused element here would be a guess.
        // SAFETY: checked above.
        unsafe { *out = ptr::null_mut() };
        S_OK
    })
}

/// Reads a window's title, bounded, as UTF-16 without a terminator.
fn window_title(window: Handle) -> Vec<u16> {
    const MAX_TITLE_UNITS: usize = 256;
    let mut buffer = vec![0_u16; MAX_TITLE_UNITS];
    // SAFETY: buffer is writable for exactly the declared count.
    let written =
        unsafe { raw::GetWindowTextW(window, buffer.as_mut_ptr(), MAX_TITLE_UNITS as i32) };
    let written = usize::try_from(written).unwrap_or(0).min(MAX_TITLE_UNITS);
    buffer.truncate(written);
    buffer
}

#[cfg(test)]
mod tests {
    use std::{ffi::c_void, ptr, sync::Arc};

    use super::{
        E_NOINTERFACE, E_POINTER, Guid, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT,
        IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT, IID_IRAW_ELEMENT_PROVIDER_SIMPLE, IID_IUNKNOWN,
        Provider, S_OK, Tree, UIA_E_NOTSUPPORTED, contain, increment, release_provider, set_focus,
    };

    fn tree() -> Arc<Tree> {
        Arc::new(Tree::new(Vec::new(), Vec::new()))
    }

    fn root() -> *mut Provider {
        Provider::create(0, None, tree())
    }

    fn child() -> *mut Provider {
        Provider::create(0, Some(0), tree())
    }

    /// Queries an interface through the object's simple vtable pointer.
    unsafe fn query_simple(provider: *mut Provider, iid: &Guid) -> (i32, *mut c_void) {
        let mut out: *mut c_void = ptr::null_mut();
        // SAFETY: a live provider, and `out` is a valid slot.
        let result = unsafe {
            ((*(*provider).simple).query_interface)(
                (&raw mut (*provider).simple).cast::<c_void>(),
                iid,
                &mut out,
            )
        };
        (result, out)
    }

    #[test]
    fn the_window_answers_all_three_interfaces() {
        let provider = root();
        for iid in [
            IID_IUNKNOWN,
            IID_IRAW_ELEMENT_PROVIDER_SIMPLE,
            IID_IRAW_ELEMENT_PROVIDER_FRAGMENT,
            IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
        ] {
            // SAFETY: a live provider.
            let (result, out) = unsafe { query_simple(provider, &iid) };
            assert_eq!(result, S_OK, "{iid:?}");
            assert!(!out.is_null());
            // SAFETY: releasing the reference the query added.
            unsafe { release_provider(provider) };
        }
        // SAFETY: releasing the creation reference.
        unsafe { release_provider(provider) };
    }

    #[test]
    fn an_element_is_not_a_fragment_root() {
        // Only the window roots the tree. An element claiming otherwise would
        // make navigation ambiguous.
        let provider = child();
        // SAFETY: a live provider.
        let (result, out) =
            unsafe { query_simple(provider, &IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT) };
        assert_eq!(result, E_NOINTERFACE);
        assert!(out.is_null());

        // SAFETY: a live provider.
        let (result, _) = unsafe { query_simple(provider, &IID_IRAW_ELEMENT_PROVIDER_FRAGMENT) };
        assert_eq!(result, S_OK);
        // SAFETY: releasing the query's reference, then the creation one.
        unsafe {
            release_provider(provider);
            release_provider(provider);
        }
    }

    #[test]
    fn a_refused_interface_clears_its_output() {
        let provider = root();
        let unsupported = Guid {
            data1: 1,
            data2: 2,
            data3: 3,
            data4: [4; 8],
        };
        // SAFETY: a live provider.
        let (result, out) = unsafe { query_simple(provider, &unsupported) };
        assert_eq!(result, E_NOINTERFACE);
        assert!(out.is_null());
        // SAFETY: releasing the creation reference.
        unsafe { release_provider(provider) };
    }

    #[test]
    fn reference_counting_frees_the_object_exactly_once() {
        let provider = root();
        // SAFETY: a live provider held by this test.
        unsafe {
            assert_eq!(increment(provider), 2);
            assert_eq!(release_provider(provider), 1);
            assert_eq!(release_provider(provider), 0);
        }
    }

    #[test]
    fn focus_cannot_be_moved_through_a_read_only_provider() {
        // SAFETY: set_focus reads nothing from its argument.
        let result = unsafe { set_focus(ptr::null_mut()) };
        assert_eq!(result, UIA_E_NOTSUPPORTED);
        assert!(result < 0);
    }

    #[test]
    fn a_panicking_method_body_fails_instead_of_aborting() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = contain(|| panic!("a provider defect must not abort the host"));
        std::panic::set_hook(previous);
        assert_eq!(result, super::E_FAIL);
        assert_eq!(contain(|| S_OK), S_OK);
    }

    #[test]
    fn every_navigation_method_rejects_a_null_output() {
        let provider = root();
        let fragment = (&raw mut unsafe { &mut *provider }.fragment).cast::<c_void>();
        // SAFETY: a live provider with deliberately null output slots.
        unsafe {
            assert_eq!(super::navigate(fragment, 3, ptr::null_mut()), E_POINTER);
            assert_eq!(super::get_runtime_id(fragment, ptr::null_mut()), E_POINTER);
            assert_eq!(
                super::get_bounding_rectangle(fragment, ptr::null_mut()),
                E_POINTER
            );
            release_provider(provider);
        }
    }
}
