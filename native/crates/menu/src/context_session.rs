use crate::{ContextMenuModel, ContextMenuRevision, MenuActionId, MenuError};

/// One revalidated semantic context-menu action for an application-facing adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextMenuActionEvent {
    revision: ContextMenuRevision,
    action: MenuActionId,
}

impl ContextMenuActionEvent {
    /// Creates one event after a current enabled context-menu item was accepted.
    #[must_use]
    pub const fn new(revision: ContextMenuRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Returns the exact current context-menu revision.
    #[must_use]
    pub const fn revision(&self) -> ContextMenuRevision {
        self.revision
    }

    /// Returns the revalidated semantic action identity.
    #[must_use]
    pub fn action(&self) -> &MenuActionId {
        &self.action
    }
}

/// Revision-bound state for one complete session context menu.
#[derive(Clone, Debug, Default)]
pub struct ContextMenuSession {
    revision: ContextMenuRevision,
    model: Option<ContextMenuModel>,
}

impl ContextMenuSession {
    /// Builds an empty context-menu session at revision zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            revision: ContextMenuRevision::INITIAL,
            model: None,
        }
    }

    /// Returns the current complete model and its revision, if one exists.
    #[must_use]
    pub fn model(&self) -> Option<(&ContextMenuModel, ContextMenuRevision)> {
        self.model.as_ref().map(|model| (model, self.revision))
    }

    /// Returns the next nonzero revision without changing the current model.
    pub fn next_revision(&self) -> Result<ContextMenuRevision, MenuError> {
        self.revision.next().ok_or(MenuError::RevisionExhausted)
    }

    /// Replaces the complete model and advances the revision without wrapping.
    pub fn replace(&mut self, model: ContextMenuModel) -> Result<ContextMenuRevision, MenuError> {
        let revision = self.next_revision()?;
        self.model = Some(model);
        self.revision = revision;
        Ok(revision)
    }

    /// Revalidates one host-derived command against the current model.
    pub fn accept_action(
        &self,
        revision: ContextMenuRevision,
        action: MenuActionId,
    ) -> Result<ContextMenuActionEvent, MenuError> {
        if revision != self.revision {
            return Err(MenuError::StaleRevision);
        }
        let model = self.model.as_ref().ok_or(MenuError::NoCurrentMenu)?;
        if !model.has_enabled_action(&action) {
            return Err(MenuError::ActionUnavailable);
        }
        Ok(ContextMenuActionEvent::new(revision, action))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ContextMenuModel, ContextMenuRevision, ContextMenuSession, MenuAction, MenuActionId,
        MenuError, MenuText,
    };

    fn action(id: &str, enabled: bool) -> MenuAction {
        MenuAction::new(
            MenuActionId::new(id).expect("fixed ID is valid"),
            MenuText::new("Command").expect("fixed label is valid"),
            enabled,
        )
    }

    #[test]
    fn accepts_only_current_enabled_context_menu_actions() {
        let mut session = ContextMenuSession::new();
        let first = session
            .replace(
                ContextMenuModel::new(vec![action("document.rename", true)])
                    .expect("model is valid"),
            )
            .expect("revision exists");
        let action_id = MenuActionId::new("document.rename").expect("fixed ID is valid");
        assert_eq!(
            session
                .accept_action(first, action_id.clone())
                .expect("action is enabled")
                .action(),
            &action_id
        );

        let second = session
            .replace(
                ContextMenuModel::new(vec![action("document.rename", false)])
                    .expect("model is valid"),
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
        assert_eq!(ContextMenuRevision::INITIAL.value(), 0);
    }
}
