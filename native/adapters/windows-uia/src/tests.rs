//! Focused verification for the Windows UI Automation provider.

use std::{
    ffi::c_void,
    ptr,
    sync::{Arc, Mutex},
};

use anodrel_ui::UiRect;
use anodrel_ui_session::{SessionInteractionCandidate, UiDocumentSession, UiInputMailbox};
use anodrel_windows_accessibility::{ClientOrigin, accessible_elements, property};

use super::{
    E_NOINTERFACE, E_POINTER, FragmentVtbl, Guid, IID_IINVOKE_PROVIDER,
    IID_IRAW_ELEMENT_PROVIDER_FRAGMENT, IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT,
    IID_IRAW_ELEMENT_PROVIDER_SIMPLE, IID_ISCROLL_ITEM_PROVIDER, IID_ISCROLL_PROVIDER,
    IID_IUNKNOWN, IID_IVALUE_PROVIDER, InvokeVtbl, Provider, S_OK, ScrollItemVtbl, ScrollVtbl,
    Tree, UIA_E_NOTSUPPORTED, UIA_INVOKE_PATTERN_ID, UIA_SCROLL_ITEM_PATTERN_ID,
    UIA_SCROLL_PATTERN_ID, UIA_VALUE_PATTERN_ID, UiAutomationActionSink, UiAutomationFocusMailbox,
    UiAutomationScrollCommand, UiAutomationScrollMailbox, UiAutomationScrollSnapshot, ValueVtbl,
    contain, increment, release_provider, set_focus,
};

const ACTION_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}}"#;
const FIELD_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"name","kind":"field","label":"Name","value":"","maxLength":64,"fontSize":16,"enabled":true}}"#;
const HIERARCHY_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"section","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"detail","kind":"text","value":"Nested","fontSize":16,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]},{"id":"footer","kind":"text","value":"Done","fontSize":16,"tone":"primary"}]}}"#;
const SCROLL_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v2","root":{"id":"viewport","kind":"scroll","child":{"id":"content","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":0,"surfaceTone":"plain","children":[{"id":"one","kind":"action","label":"One","fontSize":16,"enabled":true,"tone":"accent"},{"id":"two","kind":"action","label":"Two","fontSize":16,"enabled":true,"tone":"accent"},{"id":"three","kind":"action","label":"Three","fontSize":16,"enabled":true,"tone":"accent"}]}}}"#;

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
    let document =
        anodrel_ui_document::decode(ACTION_DOCUMENT).expect("the fixed action document is valid");
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
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
    let document =
        anodrel_ui_document::decode(ACTION_DOCUMENT).expect("the fixed action document is valid");
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
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
    let document =
        anodrel_ui_document::decode(ACTION_DOCUMENT).expect("the fixed action document is valid");
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
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
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
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

/// Builds the selected scroll group with a notifier that records each
/// closed provider command and accepts it as the host would.
fn scrollable_child(id: &str) -> (*mut Provider, Arc<Mutex<Vec<UiAutomationScrollCommand>>>) {
    let document = anodrel_ui_document::decode_v2(SCROLL_DOCUMENT)
        .expect("the fixed scroll document is valid");
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 40.0), &FixedMeasurer);
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_for_route = Arc::clone(&recorded);
    let mailbox = UiAutomationScrollMailbox::new();
    let route = mailbox.route(None);
    let completing = mailbox.clone();
    let sink = route.with_notifier(move || {
        let request = completing.take().expect("scroll request is pending");
        recorded_for_route
            .lock()
            .expect("test recording lock is available")
            .push(request.command());
        completing.complete_with(request.id(), || true).is_some()
    });
    let snapshot = UiAutomationScrollSnapshot::new(
        anodrel_ui::ElementId::new("viewport").expect("fixed ID is valid"),
        40.0,
        80.0,
        0.0,
    )
    .expect("fixed viewport overflows");
    let element = elements
        .iter()
        .position(|element| element.automation_id() == id)
        .expect("the fixed scroll element is published");
    (
        Provider::create(
            0,
            Some(element),
            Arc::new(
                Tree::new(Vec::new(), elements, Vec::new(), None, None, None).with_scroll(
                    snapshot,
                    ["content", "one", "two", "three"]
                        .into_iter()
                        .map(|id| {
                            anodrel_ui::ElementId::new(id).expect("fixed scroll item ID is valid")
                        })
                        .collect(),
                    sink,
                ),
            ),
        ),
        recorded,
    )
}

/// Builds a nested semantic tree exactly as the host publishes it.
fn hierarchy_root() -> *mut Provider {
    let document = anodrel_ui_document::decode(HIERARCHY_DOCUMENT)
        .expect("the fixed hierarchy document is valid");
    let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
    let elements = accessible_elements(
        &document.accessibility_snapshot(&layout),
        ClientOrigin::new(0, 0, 1.0),
    );
    Provider::create(
        0,
        None,
        Arc::new(Tree::new(
            Vec::new(),
            elements,
            Vec::new(),
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
    let (result, out) = unsafe { query_simple(provider, &IID_IRAW_ELEMENT_PROVIDER_FRAGMENT_ROOT) };
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
fn fragment_navigation_follows_the_published_hierarchy() {
    let root = hierarchy_root();
    let mut group = ptr::null_mut();
    let mut detail = ptr::null_mut();
    // SAFETY: each interface belongs to a live provider owned by this test,
    // and every output slot is writable. Each successful navigation adds a
    // reference that the matching release below returns.
    unsafe {
        let root_fragment = (&raw mut (*root).fragment).cast::<c_void>();
        assert_eq!(
            super::navigate(
                root_fragment,
                super::raw2::direction::FIRST_CHILD,
                &mut group
            ),
            S_OK
        );
        assert_eq!((*super::fragment_of(group)).element, Some(0));

        assert_eq!(
            super::navigate(group, super::raw2::direction::FIRST_CHILD, &mut detail),
            S_OK
        );
        assert_eq!((*super::fragment_of(detail)).element, Some(1));

        let mut nested_group = ptr::null_mut();
        assert_eq!(
            super::navigate(
                detail,
                super::raw2::direction::NEXT_SIBLING,
                &mut nested_group
            ),
            S_OK
        );
        assert_eq!((*super::fragment_of(nested_group)).element, Some(2));

        let mut nested_child = ptr::null_mut();
        assert_eq!(
            super::navigate(
                nested_group,
                super::raw2::direction::FIRST_CHILD,
                &mut nested_child,
            ),
            S_OK
        );
        assert_eq!((*super::fragment_of(nested_child)).element, Some(3));

        release_provider(super::fragment_of(nested_child));
        release_provider(super::fragment_of(nested_group));
        release_provider(super::fragment_of(detail));
        release_provider(super::fragment_of(group));
        release_provider(root);
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
    let SessionInteractionCandidate::Ui(candidate) =
        candidates.into_iter().next().expect("one action")
    else {
        panic!("Invoke must produce a document candidate");
    };
    let (candidate_revision, event) = candidate.into_parts();
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
fn only_the_selected_overflowing_group_exposes_standard_vertical_scroll() {
    let (provider, commands) = scrollable_child("viewport");
    // SAFETY: this test owns one live provider, the interface pointers it
    // queries, and all output slots passed to its COM calls.
    unsafe {
        let (result, queried) = query_simple(provider, &IID_ISCROLL_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut pattern = ptr::null_mut();
        assert_eq!(
            super::get_pattern_provider(simple, UIA_SCROLL_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const ScrollVtbl>();
        let mut horizontal = 0.0;
        let mut vertical = 0.0;
        let mut view = 0.0;
        let mut horizontal_enabled = 1;
        let mut vertical_enabled = 0;
        assert_eq!(
            ((*vtable).get_horizontal_scroll_percent)(pattern, &mut horizontal),
            S_OK
        );
        assert_eq!(
            ((*vtable).get_vertical_scroll_percent)(pattern, &mut vertical),
            S_OK
        );
        assert_eq!(((*vtable).get_vertical_view_size)(pattern, &mut view), S_OK);
        assert_eq!(
            ((*vtable).get_horizontally_scrollable)(pattern, &mut horizontal_enabled),
            S_OK
        );
        assert_eq!(
            ((*vtable).get_vertically_scrollable)(pattern, &mut vertical_enabled),
            S_OK
        );
        assert_eq!(horizontal, -1.0);
        assert_eq!(vertical, 0.0);
        assert_eq!(view, 50.0);
        assert_eq!(horizontal_enabled, 0);
        assert_eq!(vertical_enabled, 1);

        let mut property = super::Variant::empty();
        assert_eq!(
            super::get_property_value(simple, 30_055, &mut property),
            S_OK
        );
        assert_eq!(property.double_value(), Some(0.0));

        assert_eq!(((*vtable).scroll)(pattern, 2, 4), S_OK);
        assert_eq!(((*vtable).set_scroll_percent)(pattern, -1.0, 37.5), S_OK);
        assert_eq!(((*vtable).scroll)(pattern, 4, 4), UIA_E_NOTSUPPORTED);
        assert_eq!(
            ((*vtable).set_scroll_percent)(pattern, -1.0, f64::NAN),
            UIA_E_NOTSUPPORTED
        );
        release_provider(provider);
        release_provider(provider);
    }
    assert_eq!(
        *commands.lock().expect("test recording lock is available"),
        vec![
            UiAutomationScrollCommand::Line { forward: true },
            UiAutomationScrollCommand::Percent { percent: 37.5 },
        ]
    );
}

#[test]
fn an_offscreen_scroll_descendant_exposes_only_scroll_item() {
    let (provider, commands) = scrollable_child("three");
    // SAFETY: this test owns one live provider, the interface pointers it
    // queries, and all output slots passed to its COM calls.
    unsafe {
        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let mut offscreen = super::Variant::empty();
        assert_eq!(
            super::get_property_value(simple, property::IS_OFFSCREEN, &mut offscreen),
            S_OK
        );
        assert_eq!(offscreen.boolean_value(), Some(true));

        let (result, rejected) = query_simple(provider, &IID_IINVOKE_PROVIDER);
        assert_eq!(result, E_NOINTERFACE);
        assert!(rejected.is_null());

        let (result, queried) = query_simple(provider, &IID_ISCROLL_ITEM_PROVIDER);
        assert_eq!(result, S_OK);
        assert!(!queried.is_null());
        release_provider(provider);

        let mut pattern = ptr::null_mut();
        assert_eq!(
            super::get_pattern_provider(simple, UIA_SCROLL_ITEM_PATTERN_ID, &mut pattern),
            S_OK
        );
        assert!(!pattern.is_null());

        let vtable = *pattern.cast::<*const ScrollItemVtbl>();
        assert_eq!(((*vtable).scroll_into_view)(pattern), S_OK);
        release_provider(provider);
        release_provider(provider);
    }
    assert_eq!(
        *commands.lock().expect("test recording lock is available"),
        vec![UiAutomationScrollCommand::ScrollIntoView {
            item: anodrel_ui::ElementId::new("three").expect("fixed ID is valid"),
        }]
    );
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
