//! Typed native semantic candidates for one shared session interaction queue.

use anodrel_menu::{ContextMenuRevision, MenuActionId, MenuRevision, TrayRevision};
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

/// One host-mapped native context-menu interaction awaiting session validation.
#[derive(Clone, Debug)]
pub struct ContextMenuInputCandidate {
    revision: ContextMenuRevision,
    action: MenuActionId,
}

impl ContextMenuInputCandidate {
    /// Builds one candidate from a host-owned context-menu revision and semantic ID.
    #[must_use]
    pub const fn new(revision: ContextMenuRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Splits this candidate into its revision and semantic action ID.
    #[must_use]
    pub fn into_parts(self) -> (ContextMenuRevision, MenuActionId) {
        (self.revision, self.action)
    }
}

/// One host-mapped notification-area menu interaction awaiting session validation.
#[derive(Clone, Debug)]
pub struct TrayInputCandidate {
    revision: TrayRevision,
    action: MenuActionId,
}

impl TrayInputCandidate {
    /// Builds one candidate from a host-owned tray revision and semantic ID.
    #[must_use]
    pub const fn new(revision: TrayRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Splits this candidate into its revision and semantic action ID.
    #[must_use]
    pub fn into_parts(self) -> (TrayRevision, MenuActionId) {
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
    /// A local popup command derived from the host's current native-ID mapping.
    ContextMenu(ContextMenuInputCandidate),
    /// A notification-area menu command derived from the host's current native-ID mapping.
    Tray(TrayInputCandidate),
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

impl From<ContextMenuInputCandidate> for SessionInteractionCandidate {
    fn from(candidate: ContextMenuInputCandidate) -> Self {
        Self::ContextMenu(candidate)
    }
}

impl From<TrayInputCandidate> for SessionInteractionCandidate {
    fn from(candidate: TrayInputCandidate) -> Self {
        Self::Tray(candidate)
    }
}
