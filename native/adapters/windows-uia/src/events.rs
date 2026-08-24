//! One-way UI Automation event publication.
//!
//! Events begin with the host's actual focus transition. This module creates a
//! short-lived immutable provider for that new target and gives it directly to
//! Windows. It has no listener state, application callback, or retained view.

use std::{ffi::c_void, sync::Arc};

use anodrel_ui::ElementId;

use crate::{Provider, Tree, UiAutomationPublication, raw, release_provider, window_title};

/// Raises the standard focus-changed event for a current host publication.
///
/// An empty or non-focusable publication produces no provider and no Windows
/// call. Windows owns any reference it retains while handling the event; this
/// function releases only its creation reference and deliberately discards the
/// best-effort result. No caller can learn whether assistive technology heard
/// the event.
pub fn raise_focus_changed(window: raw::Handle, publication: UiAutomationPublication) {
    let Some((tree, element)) = focus_event_tree(window_title(window), publication) else {
        return;
    };
    let provider = Provider::create(window, Some(element), tree);
    // SAFETY: `provider` is live and its first field is the Simple interface
    // vtable. UiaRaiseAutomationEvent retains its own provider reference while
    // any Windows event handler processes the notification.
    unsafe {
        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let _ = raw::UiaRaiseAutomationEvent(simple, raw::UIA_AUTOMATION_FOCUS_CHANGED_EVENT_ID);
        release_provider(provider);
    }
}

/// Raises one root `ChildrenInvalidated` event after document replacement.
///
/// The caller has already accepted a newer authenticated document and released
/// its view lock. This function has no listener state or result surface.
pub fn raise_structure_changed(window: raw::Handle, publication: UiAutomationPublication) {
    let Some(tree) = structure_event_tree(window_title(window), publication) else {
        return;
    };
    let provider = Provider::create(window, None, tree);
    // SAFETY: this fresh root provider owns the creation reference. Windows may
    // retain its own reference while it delivers the best-effort notification.
    unsafe {
        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let (runtime_id, runtime_id_len) = children_invalidated_arguments();
        let _ = raw::UiaRaiseStructureChangedEvent(
            simple,
            raw::STRUCTURE_CHANGE_CHILDREN_INVALIDATED,
            runtime_id,
            runtime_id_len,
        );
        release_provider(provider);
    }
}

/// Raises one best-effort live-region event from a changed visible status.
///
/// The host has already established that a later authenticated document changed
/// its semantic status. This function rechecks that the fresh immutable tree
/// contains that status as a visible live element. It stores no listener or
/// delivery state and discards Windows' best-effort result.
pub fn raise_live_region_changed(
    window: raw::Handle,
    publication: UiAutomationPublication,
    status: &ElementId,
) {
    let Some((tree, element)) = live_region_event_tree(window_title(window), publication, status)
    else {
        return;
    };
    let provider = Provider::create(window, Some(element), tree);
    // SAFETY: `provider` is live and the standard event call may retain a
    // reference while Windows handles the outbound notification.
    unsafe {
        let simple = (&raw mut (*provider).simple).cast::<c_void>();
        let _ = raw::UiaRaiseAutomationEvent(simple, raw::UIA_LIVE_REGION_CHANGED_EVENT_ID);
        release_provider(provider);
    }
}

fn focus_event_tree(
    title: Vec<u16>,
    publication: UiAutomationPublication,
) -> Option<(Arc<Tree>, usize)> {
    let tree = publication.into_tree(title);
    let element = tree.focused()?;
    Some((tree, element))
}

fn structure_event_tree(
    title: Vec<u16>,
    publication: UiAutomationPublication,
) -> Option<Arc<Tree>> {
    let tree = publication.into_tree(title);
    (!tree.is_empty()).then_some(tree)
}

fn children_invalidated_arguments() -> (*const i32, i32) {
    (std::ptr::null(), 0)
}

fn live_region_event_tree(
    title: Vec<u16>,
    publication: UiAutomationPublication,
    status: &ElementId,
) -> Option<(Arc<Tree>, usize)> {
    let tree = publication.into_tree(title);
    let element = tree.live_region(status)?;
    Some((tree, element))
}

#[cfg(test)]
mod tests {
    use anodrel_ui::{ElementId, UiRect};
    use anodrel_ui_document::{decode, decode_v3};
    use anodrel_windows_accessibility::{ClientOrigin, accessible_elements};

    use super::{
        children_invalidated_arguments, focus_event_tree, live_region_event_tree,
        structure_event_tree,
    };
    use crate::UiAutomationPublication;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;
    const STATUS_DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v3","root":{"id":"status","kind":"status","value":"Saved","fontSize":16,"tone":"accent","politeness":"polite"}}"#;

    struct FixedMeasurer;

    impl anodrel_ui::TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, _font_size: u16) -> anodrel_ui::UiSize {
            anodrel_ui::UiSize::new(text.len() as f32 * 8.0, 18.0)
        }
    }

    #[test]
    fn an_empty_publication_never_creates_an_event_source() {
        assert!(focus_event_tree(Vec::new(), UiAutomationPublication::empty()).is_none());
        assert!(structure_event_tree(Vec::new(), UiAutomationPublication::empty()).is_none());
    }

    #[test]
    fn the_published_focus_child_is_the_event_source() {
        let document = decode(DOCUMENT).expect("the fixed document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        );
        let focus = ElementId::new("continue").expect("fixed ID is valid");
        let (_, element) = focus_event_tree(
            Vec::new(),
            UiAutomationPublication::new(elements, Vec::new(), Some(focus), None, None),
        )
        .expect("the focused action is published");

        assert_eq!(element, 2, "the event source is the focused child");
    }

    #[test]
    fn a_populated_publication_builds_a_root_structure_event_source() {
        let document = decode(DOCUMENT).expect("the fixed document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        );
        assert!(
            structure_event_tree(
                Vec::new(),
                UiAutomationPublication::new(elements, Vec::new(), None, None, None),
            )
            .is_some()
        );
    }

    #[test]
    fn children_invalidated_carries_no_provider_runtime_id_payload() {
        let (runtime_id, runtime_id_len) = children_invalidated_arguments();
        assert!(runtime_id.is_null());
        assert_eq!(runtime_id_len, 0);
    }

    #[test]
    fn a_current_visible_status_is_the_live_event_source() {
        let document = decode_v3(STATUS_DOCUMENT).expect("the status document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        );
        let status = ElementId::new("status").expect("fixed ID is valid");
        let (_, element) = live_region_event_tree(
            Vec::new(),
            UiAutomationPublication::new(elements, Vec::new(), None, None, None),
            &status,
        )
        .expect("the visible live status is published");

        assert_eq!(element, 0);
    }

    #[test]
    fn a_non_live_or_missing_status_never_creates_an_event_source() {
        let document = decode(DOCUMENT).expect("the fixed document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        );
        let status = ElementId::new("heading").expect("fixed ID is valid");

        assert!(
            live_region_event_tree(
                Vec::new(),
                UiAutomationPublication::new(elements, Vec::new(), None, None, None),
                &status,
            )
            .is_none()
        );
    }
}
