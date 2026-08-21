#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! The UI Automation provider for one Anodrel window.
//!
//! The window answers as an automation root, and the semantic elements the
//! host mapped from its current layout answer as its children. See
//! `docs/ACCESSIBILITY.md`.
//!
//! The provider is host-owned. It supplies Invoke only for an enabled button in
//! a current authenticated UI session, routes that action through the session's
//! existing bounded semantic mailbox, and supplies a read-only field value only
//! from an immutable host snapshot. A visible enabled field or button can use a
//! bounded host-owned focus route; it cannot set field text. Nothing tells an
//! application that assistive technology is listening. See Decisions 0069,
//! 0071, and 0073.

mod events;
mod focus;
mod raw;
mod raw2;
mod raw3;
mod raw4;
mod raw5;
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

use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{UiDocumentRevision, UiInputCandidate, UiInputMailbox};
use anodrel_windows_accessibility::AccessibleElement;
use raw::{
    E_FAIL, E_NOINTERFACE, E_POINTER, Guid, Handle, Hresult, IID_IRAW_ELEMENT_PROVIDER_SIMPLE,
    IID_IUNKNOWN, Lresult, S_OK, UIA_ROOT_OBJECT_ID, Variant,
};
use raw2::{
    IID_IRAW_ELEMENT_PROVIDER_FRAGMENT, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
    UIA_E_NOTSUPPORTED, UiaRect,
};
use raw3::{IID_IINVOKE_PROVIDER, UIA_INVOKE_PATTERN_ID};
use raw5::{IID_IVALUE_PROVIDER, UIA_VALUE_PATTERN_ID};
use tree::Tree;

pub use events::raise_focus_changed;
pub use focus::{
    UiAutomationFocusMailbox, UiAutomationFocusRequest, UiAutomationFocusRoute,
    UiAutomationFocusSink,
};
pub use tree::publishable;

/// The session-bound semantic route an invokable authenticated button may use.
///
/// This type deliberately holds no window handle, provider pointer, native
/// object, application callback, or mutable view. It is constructed only for a
/// non-initial authenticated-session revision and gives UI Automation exactly the
/// same bounded candidate route as local semantic input.
#[derive(Clone, Debug)]
pub struct UiAutomationActionSink {
    revision: UiDocumentRevision,
    mailbox: UiInputMailbox,
}

/// The immutable semantic data one host window publishes to UI Automation.
///
/// This packages the five snapshots and bounded routes that must remain aligned
/// for one `WM_GETOBJECT` reply. The host creates a fresh value for each reply;
/// the provider never retains a mutable native view or application callback.
pub struct UiAutomationPublication {
    elements: Vec<AccessibleElement>,
    field_values: Vec<(ElementId, String)>,
    focused: Option<ElementId>,
    action_sink: Option<UiAutomationActionSink>,
    focus_sink: Option<UiAutomationFocusSink>,
}

impl UiAutomationPublication {
    /// Builds one immutable publication from a single host layout snapshot.
    #[must_use]
    pub fn new(
        elements: Vec<AccessibleElement>,
        field_values: Vec<(ElementId, String)>,
        focused: Option<ElementId>,
        action_sink: Option<UiAutomationActionSink>,
        focus_sink: Option<UiAutomationFocusSink>,
    ) -> Self {
        Self {
            elements,
            field_values,
            focused,
            action_sink,
            focus_sink,
        }
    }

    /// Builds the empty publication for a window without a native UI document.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), None, None, None)
    }

    fn into_tree(self, title: Vec<u16>) -> Arc<Tree> {
        Arc::new(Tree::new(
            title,
            self.elements,
            self.field_values,
            self.focused,
            self.action_sink,
            self.focus_sink,
        ))
    }
}

impl UiAutomationActionSink {
    /// Builds the route for an authenticated session that has accepted a document.
    ///
    /// The initial revision has no document or layout to bind an action to, so
    /// it intentionally has no UI Automation action route.
    #[must_use]
    pub fn for_current_session(
        revision: UiDocumentRevision,
        mailbox: UiInputMailbox,
    ) -> Option<Self> {
        (revision != UiDocumentRevision::INITIAL).then_some(Self { revision, mailbox })
    }

    /// Offers one semantic button action to the existing bounded session queue.
    ///
    /// `false` means the fixed queue was full. The queue records that overflow
    /// for the granted protocol consumer; this API exposes no queue state.
    fn offer(&self, id: ElementId) -> bool {
        self.mailbox.try_push(UiInputCandidate::new(
            self.revision,
            UiEvent::ActionInvoked(id),
        ))
    }
}

/// Answers `WM_GETOBJECT` for one host window.
///
/// `elements` are the semantic elements the host mapped from the window's
/// current layout; an empty list publishes a window with no children.
/// `focused` is the host-owned focus ID from that same layout, if there is one;
/// the tree filters it to a visible, enabled, focusable published element.
/// `field_values` are copied host-owned field values from that same view; the
/// tree filters them to visible published Edit elements before exposing a
/// read-only UI Automation value.
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
    publication: UiAutomationPublication,
) -> Option<Lresult> {
    if lparam != UIA_ROOT_OBJECT_ID {
        return None;
    }
    // The provider is returned whenever the root object is asked for, without
    // first checking whether a client is listening. `UiaClientsAreListening`
    // answers whether raising an *event* is worthwhile; using it as a gate here
    // meant a window created before a screen reader started answered its early
    // requests with nothing, and was resolved to the default window provider
    // instead. Attaching a screen reader afterwards then found no semantics.

    let tree = publication.into_tree(window_title(window));
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

/// `IInvokeProvider`.
#[repr(C)]
struct InvokeVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    invoke: unsafe extern "system" fn(*mut c_void) -> Hresult,
}

/// `IValueProvider`.
#[repr(C)]
struct ValueVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    set_value: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    get_value: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> Hresult,
    get_is_read_only: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
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

static INVOKE_VTBL: InvokeVtbl = InvokeVtbl {
    query_interface: invoke_query_interface,
    add_ref: invoke_add_ref,
    release: invoke_release,
    invoke,
};

static VALUE_VTBL: ValueVtbl = ValueVtbl {
    query_interface: value_query_interface,
    add_ref: value_add_ref,
    release: value_release,
    set_value,
    get_value,
    get_is_read_only,
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
    invoke: *const InvokeVtbl,
    value: *const ValueVtbl,
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
            invoke: &raw const INVOKE_VTBL,
            value: &raw const VALUE_VTBL,
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

unsafe fn invoke_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, invoke)) }
}

unsafe fn value_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, value)) }
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
    let (is_root, element, supports_invoke, supports_value) = unsafe {
        let provider = &*provider;
        (
            provider.is_root(),
            provider.element,
            provider
                .element
                .is_some_and(|index| provider.tree.supports_invoke(index)),
            provider
                .element
                .is_some_and(|index| provider.tree.supports_value(index)),
        )
    };

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
    } else if requested == IID_IINVOKE_PROVIDER && element.is_some() && supports_invoke {
        // An Invoke interface exists only for an enabled authenticated-session
        // button. Every other provider denies it rather than exposing a method
        // that could act on a stale or diagnostic surface.
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).invoke).cast::<c_void>() }
    } else if requested == IID_IVALUE_PROVIDER && element.is_some() && supports_value {
        // A Value interface exists only for a visible Edit with a copied
        // host-owned value. It is read-only to automation below.
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).value).cast::<c_void>() }
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
forward_unknown!(
    invoke_query_interface,
    invoke_add_ref,
    invoke_release,
    invoke_of
);
forward_unknown!(
    value_query_interface,
    value_add_ref,
    value_release,
    value_of
);

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

unsafe extern "system" fn invoke(this: *mut c_void) -> Hresult {
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

unsafe extern "system" fn set_value(this: *mut c_void, _value: *const u16) -> Hresult {
    contain(|| {
        if this.is_null() {
            return E_POINTER;
        }
        // This deliberately does not read its supplied pointer. UI Automation
        // cannot turn into a second writer for host-owned field state.
        UIA_E_NOTSUPPORTED
    })
}

unsafe extern "system" fn get_value(this: *mut c_void, out: *mut *mut u16) -> Hresult {
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

unsafe extern "system" fn get_is_read_only(this: *mut c_void, out: *mut i32) -> Hresult {
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

unsafe extern "system" fn set_focus(this: *mut c_void) -> Hresult {
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

unsafe extern "system" fn get_focus(this: *mut c_void, out: *mut *mut c_void) -> Hresult {
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

    use anodrel_ui::UiRect;
    use anodrel_ui_session::{UiDocumentSession, UiInputMailbox};
    use anodrel_windows_accessibility::{ClientOrigin, accessible_elements};

    use super::{
        E_NOINTERFACE, E_POINTER, FragmentVtbl, Guid, IID_IINVOKE_PROVIDER,
        IID_IRAW_ELEMENT_PROVIDER_FRAGMENT, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
        IID_IRAW_ELEMENT_PROVIDER_SIMPLE, IID_IUNKNOWN, IID_IVALUE_PROVIDER, InvokeVtbl, Provider,
        S_OK, Tree, UIA_E_NOTSUPPORTED, UIA_INVOKE_PATTERN_ID, UIA_VALUE_PATTERN_ID,
        UiAutomationActionSink, UiAutomationFocusMailbox, ValueVtbl, contain, increment,
        release_provider, set_focus,
    };

    const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
    const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"name","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true}}"#;

    struct FixedMeasurer;

    impl anodrel_ui::TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, font_size: u16) -> anodrel_ui::UiSize {
            anodrel_ui::UiSize::new(
                text.chars().count() as f32 * f32::from(font_size) * 0.5,
                f32::from(font_size),
            )
        }
    }

    fn tree() -> Arc<Tree> {
        Arc::new(Tree::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
        ))
    }

    fn root() -> *mut Provider {
        Provider::create(0, None, tree())
    }

    fn child() -> *mut Provider {
        Provider::create(0, Some(0), tree())
    }

    /// Builds one enabled authenticated button exactly as the host publishes it.
    fn invokable_child() -> (
        *mut Provider,
        UiInputMailbox,
        anodrel_ui_session::UiDocumentRevision,
    ) {
        let document = anodrel_ui_document::decode(ACTION_DOCUMENT)
            .expect("the fixed action document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = super::publishable(accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        ));
        let mut session = UiDocumentSession::new();
        let revision = session
            .replace_document(ACTION_DOCUMENT)
            .expect("the fixed action document is valid");
        let mailbox = UiInputMailbox::new();
        let sink = UiAutomationActionSink::for_current_session(revision, mailbox.clone())
            .expect("an accepted document has an action route");
        (
            Provider::create(
                0,
                Some(0),
                Arc::new(Tree::new(
                    Vec::new(),
                    elements,
                    Vec::new(),
                    None,
                    Some(sink),
                    None,
                )),
            ),
            mailbox,
            revision,
        )
    }

    fn focused_root() -> *mut Provider {
        let document = anodrel_ui_document::decode(ACTION_DOCUMENT)
            .expect("the fixed action document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = super::publishable(accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        ));
        Provider::create(
            0,
            None,
            Arc::new(Tree::new(
                Vec::new(),
                elements,
                Vec::new(),
                Some(anodrel_ui::ElementId::new("continue").expect("fixed ID is valid")),
                None,
                None,
            )),
        )
    }

    /// Builds one visible enabled child whose host-only route accepts focus.
    fn focusable_child() -> *mut Provider {
        let document = anodrel_ui_document::decode(ACTION_DOCUMENT)
            .expect("the fixed action document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = super::publishable(accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        ));
        let mailbox = UiAutomationFocusMailbox::new();
        let route = mailbox.route(None);
        let completing = mailbox.clone();
        let sink = route.with_notifier(move || {
            let request = completing.take().expect("focus request is pending");
            completing.complete(request.id(), true)
        });
        Provider::create(
            0,
            Some(0),
            Arc::new(Tree::new(
                Vec::new(),
                elements,
                Vec::new(),
                None,
                None,
                Some(sink),
            )),
        )
    }

    fn value_child() -> *mut Provider {
        let document =
            anodrel_ui_document::decode(FIELD_DOCUMENT).expect("the fixed field document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = super::publishable(accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        ));
        Provider::create(
            0,
            Some(0),
            Arc::new(Tree::new(
                Vec::new(),
                elements,
                vec![(
                    anodrel_ui::ElementId::new("name").expect("fixed ID is valid"),
                    "Ada".to_owned(),
                )],
                None,
                None,
                None,
            )),
        )
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
    fn an_enabled_authenticated_button_exposes_and_queues_only_invoke() {
        let (provider, mailbox, revision) = invokable_child();
        // SAFETY: this test owns one live provider and writable output slots.
        unsafe {
            let (result, queried) = query_simple(provider, &IID_IINVOKE_PROVIDER);
            assert_eq!(result, S_OK);
            assert!(!queried.is_null());
            release_provider(provider);

            let simple = (&raw mut (*provider).simple).cast::<c_void>();
            let mut pattern = ptr::null_mut();
            assert_eq!(
                super::get_pattern_provider(simple, UIA_INVOKE_PATTERN_ID, &mut pattern),
                S_OK
            );
            assert!(!pattern.is_null());

            let vtable = *pattern.cast::<*const InvokeVtbl>();
            assert_eq!(((*vtable).invoke)(pattern), S_OK);
            release_provider(provider);
            release_provider(provider);
        }

        let batch = mailbox.drain();
        assert_eq!(batch.dropped(), 0);
        let candidates = batch.into_candidates();
        assert_eq!(candidates.len(), 1);
        let (candidate_revision, event) = candidates
            .into_iter()
            .next()
            .expect("one action")
            .into_parts();
        assert_eq!(candidate_revision, revision);
        assert_eq!(
            event,
            anodrel_ui::UiEvent::ActionInvoked(
                anodrel_ui::ElementId::new("continue").expect("fixed ID is valid")
            )
        );
    }

    #[test]
    fn a_field_exposes_its_value_without_an_automation_write() {
        let provider = value_child();
        // SAFETY: this test owns one live provider and writable output slots.
        unsafe {
            let (result, queried) = query_simple(provider, &IID_IVALUE_PROVIDER);
            assert_eq!(result, S_OK);
            assert!(!queried.is_null());
            release_provider(provider);

            let simple = (&raw mut (*provider).simple).cast::<c_void>();
            let mut pattern = ptr::null_mut();
            assert_eq!(
                super::get_pattern_provider(simple, UIA_VALUE_PATTERN_ID, &mut pattern),
                S_OK
            );
            assert!(!pattern.is_null());

            let vtable = *pattern.cast::<*const ValueVtbl>();
            let mut value = ptr::null_mut();
            assert_eq!(((*vtable).get_value)(pattern, &mut value), S_OK);
            assert!(!value.is_null());
            assert_eq!(super::raw::copy_and_free_bstr(value), "Ada");

            let mut read_only = 0;
            assert_eq!(((*vtable).get_is_read_only)(pattern, &mut read_only), S_OK);
            assert_eq!(read_only, 1);
            assert_eq!(
                ((*vtable).set_value)(pattern, ptr::null()),
                UIA_E_NOTSUPPORTED
            );
            release_provider(provider);
            release_provider(provider);
        }
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
    fn focus_requires_a_live_visible_provider_and_updates_its_snapshot() {
        // SAFETY: a null interface pointer has no live provider to recover.
        let result = unsafe { set_focus(ptr::null_mut()) };
        assert_eq!(result, E_POINTER);

        let root = root();
        // SAFETY: this test owns a live root provider, which is not a child
        // focus target.
        let result = unsafe { set_focus((&raw mut (*root).fragment).cast::<c_void>()) };
        assert_eq!(result, UIA_E_NOTSUPPORTED);
        assert!(result < 0);
        // SAFETY: releasing this test's creation reference.
        unsafe { release_provider(root) };

        let provider = focusable_child();
        // SAFETY: the provider is live and the route completes synchronously
        // in this test without creating a native input path.
        unsafe {
            let fragment = (&raw mut (*provider).fragment).cast::<c_void>();
            assert_eq!(set_focus(fragment), S_OK);
            assert_eq!((*provider).tree.focused(), Some(0));
            let vtable = *fragment.cast::<*const FragmentVtbl>();
            assert_eq!(((*vtable).set_focus)(fragment), S_OK);
            release_provider(provider);
        }
    }

    #[test]
    fn the_root_returns_only_its_published_focus_snapshot() {
        let provider = focused_root();
        let mut focused = ptr::null_mut();
        // SAFETY: this test owns a live root provider and a writable output
        // slot. The returned fragment owns a separate reference.
        unsafe {
            let root = (&raw mut (*provider).fragment_root).cast::<c_void>();
            assert_eq!(super::get_focus(root, &mut focused), S_OK);
            assert!(!focused.is_null());
            let focused_provider = super::fragment_of(focused);
            assert_eq!((*focused_provider).element, Some(0));
            release_provider(focused_provider);
            release_provider(provider);
        }

        let provider = root();
        let mut focused = ptr::dangling_mut::<c_void>();
        // SAFETY: the empty tree has no focused child and the output is
        // writable, so the method must clear it rather than preserve a value.
        unsafe {
            let root = (&raw mut (*provider).fragment_root).cast::<c_void>();
            assert_eq!(super::get_focus(root, &mut focused), S_OK);
            assert!(focused.is_null());
            release_provider(provider);
        }
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
            assert_eq!(
                super::get_pattern_provider(
                    (&raw mut (*provider).simple).cast::<c_void>(),
                    UIA_INVOKE_PATTERN_ID,
                    ptr::null_mut(),
                ),
                E_POINTER
            );
            assert_eq!(super::get_runtime_id(fragment, ptr::null_mut()), E_POINTER);
            assert_eq!(
                super::get_bounding_rectangle(fragment, ptr::null_mut()),
                E_POINTER
            );
            assert_eq!(
                super::get_focus(
                    (&raw mut (*provider).fragment_root).cast::<c_void>(),
                    ptr::null_mut(),
                ),
                E_POINTER
            );
            assert_eq!(
                super::get_value(
                    (&raw mut (*provider).value).cast::<c_void>(),
                    ptr::null_mut(),
                ),
                E_POINTER
            );
            assert_eq!(
                super::get_is_read_only(
                    (&raw mut (*provider).value).cast::<c_void>(),
                    ptr::null_mut(),
                ),
                E_POINTER
            );
            release_provider(provider);
        }
    }
}
