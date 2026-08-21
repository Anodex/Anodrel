//! Closed validation rules for generated project text.

use crate::init::InitError;

pub fn validate_project_slug(value: &str) -> Result<(), InitError> {
    if value.is_empty() || value.len() > 64 {
        return Err(InitError::new(
            "project slug must be 1 to 64 ASCII characters",
        ));
    }
    let bytes = value.as_bytes();
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit()
        || !bytes[bytes.len() - 1].is_ascii_lowercase() && !bytes[bytes.len() - 1].is_ascii_digit()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(InitError::new(
            "project slug must use lowercase ASCII letters, digits, and interior hyphens",
        ));
    }
    Ok(())
}

pub fn validate_display_label(value: &str) -> Result<(), InitError> {
    if value.is_empty() || value.len() > 80 || value.chars().any(char::is_control) {
        return Err(InitError::new(
            "display label must be 1 to 80 bytes without control characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_display_label, validate_project_slug};

    #[test]
    fn accepts_only_a_bounded_cargo_compatible_slug() {
        for value in ["a", "native-app", "app7", "a-7"] {
            assert!(
                validate_project_slug(value).is_ok(),
                "{value} should be valid"
            );
        }
        for value in [
            "", "-app", "app-", "App", "app_name", "app.name", "app/path",
        ] {
            assert!(
                validate_project_slug(value).is_err(),
                "{value} should be invalid"
            );
        }
    }

    #[test]
    fn accepts_only_bounded_printable_project_text() {
        assert!(validate_display_label("Anodrel Template").is_ok());
        assert!(validate_display_label("A\nB").is_err());
        assert!(validate_display_label("").is_err());
        assert!(validate_display_label(&"a".repeat(81)).is_err());
    }
}
