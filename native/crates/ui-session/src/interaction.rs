//! Typed native semantic candidates for one shared session interaction queue.

use anodrel_menu::{MenuActionId, MenuRevision};
use anodrel_ui::UiEvent;

use crate::UiDocumentRevision;

/// One native-layout-derived document interaction awaiting session validation.
#[derive(Clone, Debug)]
pub struct UiInputCandidate {
    revision: UiDocumentRevision,
    event: UiEvent,
}

impl UiInputCandidate {
    /// Builds one candidate from the revision used by a host layout and event.
    #[must_use]
    pub const fn new(revision: UiDocumentRevision, event: UiEvent) -> Self {
        Self { revision, event }
    }

    /// Splits this candidate into its revision and semantic event.
    #[must_use]
    pub fn into_parts(self) -> (UiDocumentRevision, UiEvent) {
        (self.revision, self.event)
    }
}

/// One host-mapped native-menu interaction awaiting session validation.
#[derive(Clone, Debug)]
pub struct MenuInputCandidate {
    revision: MenuRevision,
    action: MenuActionId,
}

impl MenuInputCandidate {
    /// Builds one candidate from a host-owned menu revision and semantic ID.
    #[must_use]
    pub const fn new(revision: MenuRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Splits this candidate into its revision and semantic action ID.
    #[must_use]
    pub fn into_parts(self) -> (MenuRevision, MenuActionId) {
        (self.revision, self.action)
    }
}

/// One native semantic interaction in the session's shared ordered mailbox.
#[derive(Clone, Debug)]
pub enum SessionInteractionCandidate {
    /// A document action derived from the current host layout or accessibility tree.
    Ui(UiInputCandidate),
    /// A menu command derived from the host's current native-ID mapping.
    Menu(MenuInputCandidate),
}

impl From<UiInputCandidate> for SessionInteractionCandidate {
    fn from(candidate: UiInputCandidate) -> Self {
        Self::Ui(candidate)
    }
}

impl From<MenuInputCandidate> for SessionInteractionCandidate {
    fn from(candidate: MenuInputCandidate) -> Self {
        Self::Menu(candidate)
    }
}
