use crate::{MenuAction, MenuActionId, MenuError};

/// Maximum enabled or disabled actions in one complete context-menu model.
pub const MAX_CONTEXT_MENU_ITEMS: usize = 16;

/// One exact complete semantic context-menu model.
///
/// Its display-safe action values reuse the shared menu vocabulary, but its
/// shape, revision, host service, and grant remain separate from a menu bar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextMenuModel {
    items: Vec<MenuAction>,
}

impl ContextMenuModel {
    /// Creates one bounded context-menu model with unique non-shortcut actions.
    pub fn new(items: Vec<MenuAction>) -> Result<Self, MenuError> {
        if items.is_empty() || items.len() > MAX_CONTEXT_MENU_ITEMS {
            return Err(MenuError::InvalidShape);
        }
        if items.iter().any(|item| item.shortcut().is_some()) {
            return Err(MenuError::InvalidShape);
        }
        let actions = items.iter().map(MenuAction::id).collect::<Vec<_>>();
        if actions
            .iter()
            .enumerate()
            .any(|(index, action)| actions[..index].contains(action))
        {
            return Err(MenuError::DuplicateActionId);
        }
        Ok(Self { items })
    }

    /// Returns items in their declared display order.
    #[must_use]
    pub fn items(&self) -> &[MenuAction] {
        &self.items
    }

    pub(crate) fn has_enabled_action(&self, expected: &MenuActionId) -> bool {
        self.items
            .iter()
            .any(|item| item.enabled() && item.id() == expected)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ContextMenuModel, MAX_CONTEXT_MENU_ITEMS, MenuAction, MenuActionId, MenuError,
        MenuShortcut, MenuText,
    };

    fn action(id: &str) -> MenuAction {
        MenuAction::new(
            MenuActionId::new(id).expect("fixed ID is valid"),
            MenuText::new("Command").expect("fixed label is valid"),
            true,
        )
    }

    #[test]
    fn keeps_context_models_bounded_unique_and_without_shortcuts() {
        assert_eq!(
            ContextMenuModel::new(Vec::new()),
            Err(MenuError::InvalidShape)
        );
        assert_eq!(
            ContextMenuModel::new(
                (0..=MAX_CONTEXT_MENU_ITEMS)
                    .map(|index| action(&format!("a{index}")))
                    .collect()
            ),
            Err(MenuError::InvalidShape)
        );
        assert_eq!(
            ContextMenuModel::new(vec![action("same"), action("same")]),
            Err(MenuError::DuplicateActionId)
        );
        let shortcut = MenuShortcut::parse("Ctrl+M").expect("fixed shortcut is valid");
        assert_eq!(
            ContextMenuModel::new(vec![action("shortcut").with_shortcut(shortcut)]),
            Err(MenuError::InvalidShape)
        );
    }
}
