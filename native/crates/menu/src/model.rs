use std::fmt;

use crate::MenuError;

/// Maximum top-level menus in one complete model.
pub const MAX_MENUS: usize = 8;
/// Maximum command items in one top-level menu.
pub const MAX_MENU_ITEMS: usize = 16;
/// Maximum UTF-8 bytes in a top-level menu label.
pub const MAX_MENU_LABEL_BYTES: usize = 32;
/// Maximum UTF-8 bytes in a command-item label.
pub const MAX_MENU_ITEM_LABEL_BYTES: usize = 96;
const MAX_ACTION_ID_BYTES: usize = 64;

/// One validated display label for a native menu or command.
#[derive(Clone, Eq, PartialEq)]
pub struct MenuText(String);

impl MenuText {
    /// Validates a non-control label usable by any native menu surface.
    ///
    /// Top-level menus impose their shorter limit when they are constructed.
    pub fn new(value: impl Into<String>) -> Result<Self, MenuError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_MENU_ITEM_LABEL_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(MenuError::InvalidLabel);
        }
        Ok(Self(value))
    }

    /// Returns the original validated display text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for MenuText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("MenuText").field(&self.0).finish()
    }
}

/// A validated semantic action identity for one menu command.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct MenuActionId(String);

impl MenuActionId {
    /// Validates the fixed, transport-safe menu command ID grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, MenuError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_ACTION_ID_BYTES
            && value.is_ascii()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric);
        if valid {
            Ok(Self(value))
        } else {
            Err(MenuError::InvalidActionId)
        }
    }

    /// Returns the exact semantic command identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for MenuActionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("MenuActionId")
            .field(&self.0)
            .finish()
    }
}

/// One enabled or disabled semantic command item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuAction {
    id: MenuActionId,
    label: MenuText,
    enabled: bool,
}

impl MenuAction {
    /// Creates one command from prevalidated identity and display text.
    #[must_use]
    pub const fn new(id: MenuActionId, label: MenuText, enabled: bool) -> Self {
        Self { id, label, enabled }
    }

    /// Returns the semantic action identity.
    #[must_use]
    pub fn id(&self) -> &MenuActionId {
        &self.id
    }

    /// Returns the display-safe command label.
    #[must_use]
    pub fn label(&self) -> &MenuText {
        &self.label
    }

    /// Returns whether a person may currently invoke this command.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// One top-level native menu and its command items.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Menu {
    label: MenuText,
    items: Vec<MenuAction>,
}

impl Menu {
    /// Creates one bounded non-empty top-level menu.
    pub fn new(label: MenuText, items: Vec<MenuAction>) -> Result<Self, MenuError> {
        if label.as_str().len() > MAX_MENU_LABEL_BYTES
            || items.is_empty()
            || items.len() > MAX_MENU_ITEMS
        {
            return Err(MenuError::InvalidShape);
        }
        Ok(Self { label, items })
    }

    /// Returns the display-safe top-level label.
    #[must_use]
    pub fn label(&self) -> &MenuText {
        &self.label
    }

    /// Returns command items in their declared display order.
    #[must_use]
    pub fn items(&self) -> &[MenuAction] {
        &self.items
    }
}

/// One exact complete native-session menu model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuModel {
    menus: Vec<Menu>,
}

impl MenuModel {
    /// Creates one bounded menu model with globally unique command identities.
    pub fn new(menus: Vec<Menu>) -> Result<Self, MenuError> {
        if menus.is_empty() || menus.len() > MAX_MENUS {
            return Err(MenuError::InvalidShape);
        }
        let actions = menus
            .iter()
            .flat_map(Menu::items)
            .map(MenuAction::id)
            .collect::<Vec<_>>();
        if actions.len() > MAX_MENUS * MAX_MENU_ITEMS {
            return Err(MenuError::InvalidShape);
        }
        if actions
            .iter()
            .enumerate()
            .any(|(index, action)| actions[..index].contains(action))
        {
            return Err(MenuError::DuplicateActionId);
        }
        Ok(Self { menus })
    }

    /// Returns top-level menus in their declared display order.
    #[must_use]
    pub fn menus(&self) -> &[Menu] {
        &self.menus
    }

    pub(crate) fn has_enabled_action(&self, expected: &MenuActionId) -> bool {
        self.menus
            .iter()
            .flat_map(Menu::items)
            .any(|action| action.enabled() && action.id() == expected)
    }
}
