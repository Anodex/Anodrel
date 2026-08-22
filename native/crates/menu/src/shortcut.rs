use crate::MenuError;

/// One validated canonical local menu-shortcut declaration.
///
/// This portable value has no operating-system key code. A host maps its own
/// keyboard message to the small ASCII key set before matching it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MenuShortcut {
    key: u8,
    shift: bool,
}

impl MenuShortcut {
    /// Parses only `Ctrl+<A-Z0-9>` or `Ctrl+Shift+<A-Z0-9>` exactly.
    pub fn parse(value: &str) -> Result<Self, MenuError> {
        let (shift, key) = if let Some(key) = value.strip_prefix("Ctrl+Shift+") {
            (true, key)
        } else if let Some(key) = value.strip_prefix("Ctrl+") {
            (false, key)
        } else {
            return Err(MenuError::InvalidShortcut);
        };
        let [key] = key.as_bytes() else {
            return Err(MenuError::InvalidShortcut);
        };
        let key = *key;
        if !(key.is_ascii_uppercase() || key.is_ascii_digit()) {
            return Err(MenuError::InvalidShortcut);
        }
        Ok(Self { key, shift })
    }

    /// Returns the host-display spelling of this canonical shortcut.
    #[must_use]
    pub fn display_text(&self) -> String {
        let key = char::from(self.key);
        if self.shift {
            format!("Ctrl+Shift+{key}")
        } else {
            format!("Ctrl+{key}")
        }
    }

    /// Reports whether one host-normalized local key press matches this value.
    #[must_use]
    pub const fn matches_key_press(
        &self,
        key: u8,
        control_down: bool,
        shift_down: bool,
        alt_down: bool,
    ) -> bool {
        control_down && !alt_down && shift_down == self.shift && key == self.key
    }
}

#[cfg(test)]
mod tests {
    use crate::{MenuError, MenuShortcut};

    #[test]
    fn accepts_only_canonical_control_shortcuts() {
        assert_eq!(
            MenuShortcut::parse("Ctrl+S")
                .expect("canonical shortcut is valid")
                .display_text(),
            "Ctrl+S"
        );
        assert_eq!(
            MenuShortcut::parse("Ctrl+Shift+1")
                .expect("canonical shifted shortcut is valid")
                .display_text(),
            "Ctrl+Shift+1"
        );
        for invalid in [
            "ctrl+S",
            "Ctrl+s",
            "Shift+Ctrl+S",
            "Ctrl+Alt+S",
            "Ctrl+F1",
            "Ctrl+.",
            "Ctrl+SS",
            "Ctrl+ S",
            "Ctrl+Shift+",
        ] {
            assert_eq!(
                MenuShortcut::parse(invalid),
                Err(MenuError::InvalidShortcut),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn matches_only_the_declared_modifier_state_and_key() {
        let shortcut = MenuShortcut::parse("Ctrl+Shift+M").expect("shortcut is valid");
        assert!(shortcut.matches_key_press(b'M', true, true, false));
        assert!(!shortcut.matches_key_press(b'M', true, false, false));
        assert!(!shortcut.matches_key_press(b'M', true, true, true));
        assert!(!shortcut.matches_key_press(b'M', false, true, false));
        assert!(!shortcut.matches_key_press(b'N', true, true, false));
    }
}
