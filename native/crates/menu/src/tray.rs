//! Revision-bound semantic tray-menu state for one authenticated session.

use std::fmt;

use crate::{ContextMenuModel, MenuActionId, MenuError};

/// A nonzero monotonic revision of one complete tray menu.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TrayRevision(u64);

impl TrayRevision {
    /// The empty session's initial tray revision.
    pub const INITIAL: Self = Self(0);

    /// Returns the next revision, or `None` instead of wrapping.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Renders the revision as its canonical decimal protocol value.
    #[must_use]
    pub fn as_decimal(self) -> String {
        self.0.to_string()
    }

    /// Returns the portable revision value for ordering and tests.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Default for TrayRevision {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl fmt::Display for TrayRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One revalidated semantic tray action ready for event delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayActionEvent {
    revision: TrayRevision,
    action: MenuActionId,
}

impl TrayActionEvent {
    /// Creates one event after the current enabled tray item was accepted.
    #[must_use]
    pub const fn new(revision: TrayRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Returns the exact complete tray revision that produced this action.
    #[must_use]
    pub const fn revision(&self) -> TrayRevision {
        self.revision
    }

    /// Returns the semantic action identity.
    #[must_use]
    pub fn action(&self) -> &MenuActionId {
        &self.action
    }
}

/// Revision-bound state for one complete session tray menu.
#[derive(Clone, Debug, Default)]
pub struct TraySession {
    revision: TrayRevision,
    model: Option<ContextMenuModel>,
}

impl TraySession {
    /// Builds an empty tray session at revision zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            revision: TrayRevision::INITIAL,
            model: None,
        }
    }

    /// Returns the current complete model and its revision, if one exists.
    #[must_use]
    pub fn model(&self) -> Option<(&ContextMenuModel, TrayRevision)> {
        self.model.as_ref().map(|model| (model, self.revision))
    }

    /// Returns the next nonzero revision without changing the model.
    pub fn next_revision(&self) -> Result<TrayRevision, MenuError> {
        self.revision.next().ok_or(MenuError::RevisionExhausted)
    }

    /// Replaces the complete model and advances the revision without wrapping.
    pub fn replace(&mut self, model: ContextMenuModel) -> Result<TrayRevision, MenuError> {
        let revision = self.next_revision()?;
        self.model = Some(model);
        self.revision = revision;
        Ok(revision)
    }

    /// Revalidates one host-derived menu command against the current tray.
    pub fn accept_action(
        &self,
        revision: TrayRevision,
        action: MenuActionId,
    ) -> Result<TrayActionEvent, MenuError> {
        if revision != self.revision {
            return Err(MenuError::StaleRevision);
        }
        let model = self.model.as_ref().ok_or(MenuError::NoCurrentMenu)?;
        if !model.has_enabled_action(&action) {
            return Err(MenuError::ActionUnavailable);
        }
        Ok(TrayActionEvent::new(revision, action))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ContextMenuModel, MenuAction, MenuActionId, MenuError, MenuText, TrayRevision, TraySession,
    };

    fn action(id: &str, enabled: bool) -> MenuAction {
        MenuAction::new(
            MenuActionId::new(id).expect("fixed ID is valid"),
            MenuText::new("Command").expect("fixed label is valid"),
            enabled,
        )
    }

    #[test]
    fn accepts_only_current_enabled_tray_actions() {
        let mut session = TraySession::new();
        let first = session
            .replace(
                ContextMenuModel::new(vec![action("window.open", true)]).expect("model is valid"),
            )
            .expect("revision exists");
        let action_id = MenuActionId::new("window.open").expect("fixed ID is valid");
        assert_eq!(
            session
                .accept_action(first, action_id.clone())
                .expect("action is enabled")
                .action(),
            &action_id
        );

        let second = session
            .replace(
                ContextMenuModel::new(vec![action("window.open", false)]).expect("model is valid"),
            )
            .expect("revision exists");
        assert_eq!(
            session.accept_action(first, action_id.clone()),
            Err(MenuError::StaleRevision)
        );
        assert_eq!(
            session.accept_action(second, action_id),
            Err(MenuError::ActionUnavailable)
        );
        assert_eq!(TrayRevision::INITIAL.value(), 0);
    }
}
