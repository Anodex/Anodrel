use std::fmt;

/// A safe validation or revision failure for the portable menu model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuError {
    /// A label was empty, too large, or contained a control character.
    InvalidLabel,
    /// A semantic command ID did not follow the fixed menu grammar.
    InvalidActionId,
    /// A shortcut was not one of the small canonical local forms.
    InvalidShortcut,
    /// A menu model was empty or exceeded its fixed menu or item bounds.
    InvalidShape,
    /// Two command items named the same semantic action ID.
    DuplicateActionId,
    /// Two command items declared the same canonical shortcut.
    DuplicateShortcut,
    /// The monotonic revision space cannot advance further.
    RevisionExhausted,
    /// An action was offered under an older or newer menu revision.
    StaleRevision,
    /// No menu model is currently installed in this portable session.
    NoCurrentMenu,
    /// The named action is absent or disabled in the current menu model.
    ActionUnavailable,
}

impl fmt::Display for MenuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLabel => "menu label is invalid",
            Self::InvalidActionId => "menu action ID is invalid",
            Self::InvalidShortcut => "menu shortcut is invalid",
            Self::InvalidShape => "menu model shape is invalid",
            Self::DuplicateActionId => "menu action ID is duplicated",
            Self::DuplicateShortcut => "menu shortcut is duplicated",
            Self::RevisionExhausted => "menu revision space is exhausted",
            Self::StaleRevision => "menu action belongs to a stale revision",
            Self::NoCurrentMenu => "no current menu exists",
            Self::ActionUnavailable => "menu action is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for MenuError {}
