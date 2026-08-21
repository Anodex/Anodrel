use crate::{MenuActionId, MenuError, MenuModel, MenuRevision};

/// One revalidated semantic menu action for an application-facing adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuActionEvent {
    revision: MenuRevision,
    action: MenuActionId,
}

impl MenuActionEvent {
    /// Creates one event after a current enabled command was accepted.
    #[must_use]
    pub const fn new(revision: MenuRevision, action: MenuActionId) -> Self {
        Self { revision, action }
    }

    /// Returns the exact current menu revision.
    #[must_use]
    pub const fn revision(&self) -> MenuRevision {
        self.revision
    }

    /// Returns the revalidated semantic command identity.
    #[must_use]
    pub fn action(&self) -> &MenuActionId {
        &self.action
    }
}

/// Revision-bound state for one current complete session menu.
#[derive(Clone, Debug, Default)]
pub struct MenuSession {
    revision: MenuRevision,
    model: Option<MenuModel>,
}

impl MenuSession {
    /// Builds an empty menu session at revision zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            revision: MenuRevision::INITIAL,
            model: None,
        }
    }

    /// Returns the current complete model and its revision, if one exists.
    #[must_use]
    pub fn model(&self) -> Option<(&MenuModel, MenuRevision)> {
        self.model.as_ref().map(|model| (model, self.revision))
    }

    /// Replaces the complete model and advances the revision without wrapping.
    pub fn replace(&mut self, model: MenuModel) -> Result<MenuRevision, MenuError> {
        let revision = self.revision.next().ok_or(MenuError::RevisionExhausted)?;
        self.model = Some(model);
        self.revision = revision;
        Ok(revision)
    }

    /// Removes the current model and invalidates outstanding action candidates.
    ///
    /// Returns `Ok(None)` if no model was installed.
    pub fn clear(&mut self) -> Result<Option<MenuRevision>, MenuError> {
        if self.model.is_none() {
            return Ok(None);
        }
        let revision = self.revision.next().ok_or(MenuError::RevisionExhausted)?;
        self.model = None;
        self.revision = revision;
        Ok(Some(revision))
    }

    /// Revalidates one host-derived command against the current model.
    pub fn accept_action(
        &self,
        revision: MenuRevision,
        action: MenuActionId,
    ) -> Result<MenuActionEvent, MenuError> {
        if revision != self.revision {
            return Err(MenuError::StaleRevision);
        }
        let model = self.model.as_ref().ok_or(MenuError::NoCurrentMenu)?;
        if !model.has_enabled_action(&action) {
            return Err(MenuError::ActionUnavailable);
        }
        Ok(MenuActionEvent::new(revision, action))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        MAX_MENU_ITEM_LABEL_BYTES, Menu, MenuAction, MenuActionId, MenuError, MenuModel,
        MenuRevision, MenuSession, MenuText,
    };

    fn text(value: &str) -> MenuText {
        MenuText::new(value).expect("test label is valid")
    }

    fn action(id: &str, enabled: bool) -> MenuAction {
        MenuAction::new(
            MenuActionId::new(id).expect("test ID is valid"),
            text("Command"),
            enabled,
        )
    }

    fn model(enabled: bool) -> MenuModel {
        MenuModel::new(vec![
            Menu::new(text("File"), vec![action("document.new", enabled)])
                .expect("test menu is valid"),
        ])
        .expect("test model is valid")
    }

    #[test]
    fn validates_labels_action_ids_and_global_action_uniqueness() {
        assert_eq!(MenuText::new(""), Err(MenuError::InvalidLabel));
        assert_eq!(MenuText::new("line\nbreak"), Err(MenuError::InvalidLabel));
        assert_eq!(
            MenuText::new("x".repeat(MAX_MENU_ITEM_LABEL_BYTES + 1)),
            Err(MenuError::InvalidLabel)
        );
        assert_eq!(
            MenuActionId::new(".invalid"),
            Err(MenuError::InvalidActionId)
        );
        assert_eq!(
            MenuActionId::new("has space"),
            Err(MenuError::InvalidActionId)
        );
        let duplicate = MenuModel::new(vec![
            Menu::new(text("File"), vec![action("same", true)]).expect("test menu is valid"),
            Menu::new(text("Edit"), vec![action("same", true)]).expect("test menu is valid"),
        ]);
        assert_eq!(duplicate, Err(MenuError::DuplicateActionId));
        assert_eq!(
            Menu::new(
                MenuText::new("x".repeat(33)).expect("label is valid"),
                vec![action("top", true)],
            ),
            Err(MenuError::InvalidShape)
        );
    }

    #[test]
    fn replaces_and_revalidates_only_current_enabled_actions() {
        let mut session = MenuSession::new();
        let first = session.replace(model(true)).expect("first menu is valid");
        assert_eq!(first.value(), 1);
        let action = MenuActionId::new("document.new").expect("test ID is valid");
        assert_eq!(
            session
                .accept_action(first, action.clone())
                .expect("action is enabled")
                .action(),
            &action
        );

        let second = session.replace(model(false)).expect("second menu is valid");
        assert_eq!(
            session.accept_action(first, action.clone()),
            Err(MenuError::StaleRevision)
        );
        assert_eq!(
            session.accept_action(second, action),
            Err(MenuError::ActionUnavailable)
        );
        assert_eq!(
            session.clear(),
            Ok(Some(
                MenuRevision::INITIAL
                    .next()
                    .unwrap()
                    .next()
                    .unwrap()
                    .next()
                    .unwrap()
            ))
        );
        assert_eq!(session.clear(), Ok(None));
    }
}
