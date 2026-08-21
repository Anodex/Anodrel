//! One-way UI Automation event publication.
//!
//! Events begin with the host's actual focus transition. This module creates a
//! short-lived immutable provider for that new target and gives it directly to
//! Windows. It has no listener state, application callback, or retained view.

use std::{ffi::c_void, sync::Arc};

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

fn focus_event_tree(
    title: Vec<u16>,
    publication: UiAutomationPublication,
) -> Option<(Arc<Tree>, usize)> {
    let tree = publication.into_tree(title);
    let element = tree.focused()?;
    Some((tree, element))
}

#[cfg(test)]
mod tests {
    use anodrel_ui::{ElementId, UiRect};
    use anodrel_ui_document::decode;
    use anodrel_windows_accessibility::{ClientOrigin, accessible_elements};

    use super::focus_event_tree;
    use crate::{UiAutomationPublication, publishable};

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":0,"top":0,"right":0,"bottom":0},"gap":10,"surfaceTone":"plain","children":[{"id":"heading","kind":"text","value":"Anodrel","fontSize":16,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#;

    struct FixedMeasurer;

    impl anodrel_ui::TextMeasurer for FixedMeasurer {
        fn measure(&self, text: &str, _font_size: u16) -> anodrel_ui::UiSize {
            anodrel_ui::UiSize::new(text.len() as f32 * 8.0, 18.0)
        }
    }

    #[test]
    fn an_empty_publication_never_creates_an_event_source() {
        assert!(focus_event_tree(Vec::new(), UiAutomationPublication::empty()).is_none());
    }

    #[test]
    fn the_published_focus_child_is_the_event_source() {
        let document = decode(DOCUMENT).expect("the fixed document is valid");
        let layout = document.layout(UiRect::new(0.0, 0.0, 400.0, 300.0), &FixedMeasurer);
        let elements = publishable(accessible_elements(
            &document.accessibility_snapshot(&layout),
            ClientOrigin::new(0, 0, 1.0),
        ));
        let focus = ElementId::new("continue").expect("fixed ID is valid");
        let (_, element) = focus_event_tree(
            Vec::new(),
            UiAutomationPublication::new(elements, Vec::new(), Some(focus), None, None),
        )
        .expect("the focused action is published");

        assert_eq!(element, 1, "the event source is the focused child");
    }
}
