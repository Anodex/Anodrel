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
mod interfaces;
mod lifetime;
mod patterns;
