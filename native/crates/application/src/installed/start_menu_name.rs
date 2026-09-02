//! One signed Windows-safe filename for a future Start-menu link.

use std::fmt;

use super::product::is_safe_display_text;

/// A signed single filename component for the fixed Windows Start-menu link.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartMenuName(String);

/// A signed Start-menu name did not meet the Windows filename grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartMenuNameError {
    /// The name was not one safe Windows filename component.
    Invalid,
}

impl fmt::Display for StartMenuNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Start-menu name is invalid")
    }
}

impl std::error::Error for StartMenuNameError {}

impl StartMenuName {
    /// Creates a signed name safe for one Windows `.lnk` filename component.
    pub fn new(value: &str) -> Result<Self, StartMenuNameError> {
        is_safe_start_menu_name(value)
            .then(|| Self(value.to_owned()))
            .ok_or(StartMenuNameError::Invalid)
    }

    /// Returns the signed Start-menu filename stem.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_safe_start_menu_name(value: &str) -> bool {
    is_safe_display_text(value)
        && !value.ends_with('.')
        && !value.chars().any(|character| {
            matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        })
        && !is_windows_device_name(value)
}

fn is_windows_device_name(value: &str) -> bool {
    let base = value.split('.').next().unwrap_or_default();
    matches!(
        base.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::{StartMenuName, StartMenuNameError};

    #[test]
    fn accepts_one_human_facing_windows_filename_component() {
        let name = StartMenuName::new("Anodrel Sample").expect("safe name parses");
        assert_eq!(name.as_str(), "Anodrel Sample");
    }

    #[test]
    fn rejects_reserved_or_unsafe_windows_filename_components() {
        for name in [
            ".",
            "..",
            "Anodrel.",
            "Anodrel/Sample",
            "Anodrel:Sample",
            "CON",
            "lPt9.tools",
        ] {
            assert_eq!(StartMenuName::new(name), Err(StartMenuNameError::Invalid));
        }
    }
}
