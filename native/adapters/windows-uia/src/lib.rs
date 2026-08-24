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
mod fragments;
mod patterns;
mod raw;
mod raw2;
mod raw3;
mod raw4;
mod raw5;
mod raw6;
mod raw7;
mod scroll;
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
pub(crate) use fragments::window_title;
use fragments::*;
use patterns::*;
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
use raw6::{
    IID_ISCROLL_PROVIDER, SCROLL_AMOUNT_LARGE_DECREMENT, SCROLL_AMOUNT_LARGE_INCREMENT,
    SCROLL_AMOUNT_NO_AMOUNT, SCROLL_AMOUNT_SMALL_DECREMENT, SCROLL_AMOUNT_SMALL_INCREMENT,
    UIA_SCROLL_PATTERN_ID, UIA_SCROLL_PATTERN_NO_SCROLL,
};
use raw7::{IID_ISCROLL_ITEM_PROVIDER, UIA_SCROLL_ITEM_PATTERN_ID};
use tree::Tree;

pub use events::{raise_focus_changed, raise_live_region_changed, raise_structure_changed};
pub use focus::{
    UiAutomationFocusMailbox, UiAutomationFocusRequest, UiAutomationFocusRoute,
    UiAutomationFocusSink,
};
pub use scroll::{
    UiAutomationScrollCommand, UiAutomationScrollMailbox, UiAutomationScrollRequest,
    UiAutomationScrollRoute, UiAutomationScrollSink, UiAutomationScrollSnapshot,
};

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
/// This packages the scroll snapshot, permitted item identities, and bounded
/// routes that must remain aligned
/// for one `WM_GETOBJECT` reply. The host creates a fresh value for each reply;
/// the provider never retains a mutable native view or application callback.
pub struct UiAutomationPublication {
    elements: Vec<AccessibleElement>,
    field_values: Vec<(ElementId, String)>,
    focused: Option<ElementId>,
    action_sink: Option<UiAutomationActionSink>,
    focus_sink: Option<UiAutomationFocusSink>,
    scroll_snapshot: Option<UiAutomationScrollSnapshot>,
    scroll_items: Vec<ElementId>,
    scroll_sink: Option<UiAutomationScrollSink>,
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
            scroll_snapshot: None,
            scroll_items: Vec::new(),
            scroll_sink: None,
        }
    }

    /// Builds the empty publication for a window without a native UI document.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), None, None, None)
    }

    /// Adds the one host-selected vertical scroll snapshot, item set, and route.
    ///
    /// The host calls this only when the same current layout has a first
    /// overflowing viewport. The provider keeps those immutable values paired
    /// with its bounded route; it cannot look up a mutable view later.
    #[must_use]
    pub fn with_scroll(
        mut self,
        snapshot: UiAutomationScrollSnapshot,
        items: Vec<ElementId>,
        sink: UiAutomationScrollSink,
    ) -> Self {
        self.scroll_snapshot = Some(snapshot);
        self.scroll_items = items;
        self.scroll_sink = Some(sink);
        self
    }

    fn into_tree(self, title: Vec<u16>) -> Arc<Tree> {
        let Self {
            elements,
            field_values,
            focused,
            action_sink,
            focus_sink,
            scroll_snapshot,
            scroll_items,
            scroll_sink,
        } = self;
        let tree = Tree::new(
            title,
            elements,
            field_values,
            focused,
            action_sink,
            focus_sink,
        );
        let tree = match (scroll_snapshot, scroll_sink) {
            (Some(snapshot), Some(sink)) => tree.with_scroll(snapshot, scroll_items, sink),
            _ => tree,
        };
        Arc::new(tree)
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

/// `IScrollProvider`.
#[repr(C)]
struct ScrollVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    scroll: unsafe extern "system" fn(*mut c_void, i32, i32) -> Hresult,
    set_scroll_percent: unsafe extern "system" fn(*mut c_void, f64, f64) -> Hresult,
    get_horizontal_scroll_percent: unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    get_vertical_scroll_percent: unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    get_horizontal_view_size: unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    get_vertical_view_size: unsafe extern "system" fn(*mut c_void, *mut f64) -> Hresult,
    get_horizontally_scrollable: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
    get_vertically_scrollable: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hresult,
}

/// `IScrollItemProvider`.
#[repr(C)]
struct ScrollItemVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    scroll_into_view: unsafe extern "system" fn(*mut c_void) -> Hresult,
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

static SCROLL_VTBL: ScrollVtbl = ScrollVtbl {
    query_interface: scroll_query_interface,
    add_ref: scroll_add_ref,
    release: scroll_release,
    scroll,
    set_scroll_percent,
    get_horizontal_scroll_percent,
    get_vertical_scroll_percent,
    get_horizontal_view_size,
    get_vertical_view_size,
    get_horizontally_scrollable,
    get_vertically_scrollable,
};

static SCROLL_ITEM_VTBL: ScrollItemVtbl = ScrollItemVtbl {
    query_interface: scroll_item_query_interface,
    add_ref: scroll_item_add_ref,
    release: scroll_item_release,
    scroll_into_view,
};

/// One reference-counted provider for the window root or one of its elements.
///
/// COM reaches an object through the vtable pointer for the interface being
/// used, so all interface vtable fields sit at the front and each method recovers the object by
/// subtracting its own field offset. Every other field is set once at creation.
#[repr(C)]
struct Provider {
    simple: *const SimpleVtbl,
    fragment: *const FragmentVtbl,
    fragment_root: *const FragmentRootVtbl,
    invoke: *const InvokeVtbl,
    value: *const ValueVtbl,
    scroll: *const ScrollVtbl,
    scroll_item: *const ScrollItemVtbl,
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
            scroll: &raw const SCROLL_VTBL,
            scroll_item: &raw const SCROLL_ITEM_VTBL,
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

unsafe fn scroll_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, scroll)) }
}

unsafe fn scroll_item_of(this: *mut c_void) -> *mut Provider {
    unsafe { provider_from(this, offset_of!(Provider, scroll_item)) }
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
    let (is_root, element, supports_invoke, supports_value, supports_scroll, supports_scroll_item) = unsafe {
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
            provider
                .element
                .is_some_and(|index| provider.tree.supports_scroll(index)),
            provider
                .element
                .is_some_and(|index| provider.tree.supports_scroll_item(index)),
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
    } else if requested == IID_ISCROLL_PROVIDER && element.is_some() && supports_scroll {
        // A Scroll interface exists only for the one selected overflowing Group
        // and only while this immutable publication carries its host-only route.
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).scroll).cast::<c_void>() }
    } else if requested == IID_ISCROLL_ITEM_PROVIDER && element.is_some() && supports_scroll_item {
        // A ScrollItem interface exists only for an immutable descendant the
        // host bound to this same selected viewport. It cannot choose a
        // viewport or operate another view.
        // SAFETY: as above.
        unsafe { (&raw mut (*provider).scroll_item).cast::<c_void>() }
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
forward_unknown!(
    scroll_query_interface,
    scroll_add_ref,
    scroll_release,
    scroll_of
);
forward_unknown!(
    scroll_item_query_interface,
    scroll_item_add_ref,
    scroll_item_release,
    scroll_item_of
);

#[cfg(test)]
mod tests;
