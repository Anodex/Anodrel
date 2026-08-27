//! Immutable semantic data and action routing for one UI Automation reply.

use std::sync::Arc;

use anodrel_ui::{ElementId, UiEvent};
use anodrel_ui_session::{UiDocumentRevision, UiInputCandidate, UiInputMailbox};
use anodrel_windows_accessibility::AccessibleElement;

use crate::{Tree, UiAutomationFocusSink, UiAutomationScrollSink, UiAutomationScrollSnapshot};

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
/// routes that must remain aligned for one `WM_GETOBJECT` reply. The host creates
/// a fresh value for each reply; the provider never retains a mutable native view
/// or application callback.
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

    pub(crate) fn into_tree(self, title: Vec<u16>) -> Arc<Tree> {
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
    pub(crate) fn offer(&self, id: ElementId) -> bool {
        self.mailbox.try_push(UiInputCandidate::new(
            self.revision,
            UiEvent::ActionInvoked(id),
        ))
    }
}
