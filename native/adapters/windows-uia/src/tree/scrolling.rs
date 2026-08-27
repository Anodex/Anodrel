//! The bounded UI Automation scroll and scroll-item provider surface.

use std::collections::BTreeSet;

use anodrel_ui::ElementId;
use anodrel_windows_accessibility::control_type;

use crate::{UiAutomationScrollCommand, UiAutomationScrollSink, UiAutomationScrollSnapshot};

use super::Tree;

/// The one host-selected vertical viewport that this immutable tree may expose.
#[derive(Debug)]
pub(super) struct ScrollCapability {
    pub(super) snapshot: UiAutomationScrollSnapshot,
    pub(super) items: BTreeSet<ElementId>,
    pub(super) sink: UiAutomationScrollSink,
}

impl Tree {
    /// Whether one published Group exposes the bounded vertical ScrollPattern.
    #[must_use]
    pub(crate) fn supports_scroll(&self, index: usize) -> bool {
        self.scroll_snapshot(index).is_some()
    }

    /// Whether one published descendant exposes `ScrollItemPattern`.
    ///
    /// The selected viewport itself remains only a Scroll provider. Each
    /// accepted child identity came from the host's nearest-scroll-ancestor
    /// walk, so a provider cannot use this route to select another viewport.
    #[must_use]
    pub(crate) fn supports_scroll_item(&self, index: usize) -> bool {
        self.scroll_item_target(index).is_some()
    }

    /// Returns the published immutable scroll values for one selected group.
    #[must_use]
    pub(crate) fn scroll_snapshot(&self, index: usize) -> Option<&UiAutomationScrollSnapshot> {
        let capability = self.scroll.as_ref()?;
        let element = self.elements.get(index)?;
        (element.control_type() == control_type::GROUP
            && element.automation_id() == capability.snapshot.target().as_str())
        .then_some(&capability.snapshot)
    }

    /// Offers one closed vertical command to the selected host-owned viewport.
    ///
    /// `false` combines every generic refusal. The provider has no route to an
    /// application, a pointer stream, another viewport, or a failure detail.
    pub(crate) fn scroll(&self, index: usize, command: UiAutomationScrollCommand) -> bool {
        let capability = match (self.scroll.as_ref(), self.scroll_snapshot(index)) {
            (Some(capability), Some(_)) => capability,
            _ => return false,
        };
        capability
            .sink
            .scroll(capability.snapshot.target().clone(), command)
    }

    /// Asks the selected viewport to reveal one of its bounded descendants.
    ///
    /// A fully clipped element may use this one route, but it cannot gain
    /// focus, invocation, or a field-value read merely by appearing in the
    /// immutable navigation tree.
    pub(crate) fn scroll_into_view(&self, index: usize) -> bool {
        let Some(target) = self.scroll_item_target(index) else {
            return false;
        };
        let Some(capability) = &self.scroll else {
            return false;
        };
        capability.sink.scroll(
            capability.snapshot.target().clone(),
            UiAutomationScrollCommand::ScrollIntoView { item: target },
        )
    }

    fn scroll_item_target(&self, index: usize) -> Option<ElementId> {
        let capability = self.scroll.as_ref()?;
        let element = self.elements.get(index)?;
        let id = ElementId::new(element.automation_id()).ok()?;
        (id.as_str() != capability.snapshot.target().as_str() && capability.items.contains(&id))
            .then_some(id)
    }
}
