//! Bounded, validated external HTTPS link values.
//!
//! This crate owns no native browser, shell, process, or network operation.
//! Native adapters receive only a previously validated link. See
//! `docs/EXTERNAL_LINKS.md` and Decision 0042.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

/// Maximum bytes in one external link URL.
pub const MAX_EXTERNAL_LINK_BYTES: usize = 2_048;

/// One validated HTTPS address suitable for external operating-system handoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalLink(String);

impl ExternalLink {
    /// Parses one bounded HTTPS address with a strict DNS-style authority.
    pub fn parse(value: impl Into<String>) -> Result<Self, ExternalLinkInputError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ExternalLinkInputError::Empty);
        }
        if value.len() > MAX_EXTERNAL_LINK_BYTES {
            return Err(ExternalLinkInputError::TooLong);
        }
        if !value.is_ascii()
            || value
                .bytes()
                .any(|byte| !byte.is_ascii_graphic() || byte == b'\\')
        {
            return Err(ExternalLinkInputError::InvalidCharacter);
        }
        let Some(remainder) = value.strip_prefix("https://") else {
            return Err(ExternalLinkInputError::UnsupportedScheme);
        };
        let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
        let authority = &remainder[..authority_end];
        validate_authority(authority)?;
        Ok(Self(value))
    }

    /// Returns the exact validated URL without normalization or rewriting.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The portable service boundary used by a host core.
///
/// Implementations own the operating-system handoff. They must not construct
/// shell commands, retain process handles, or log a link value.
pub trait ExternalLinkService: fmt::Debug + Send {
    /// Hands one previously validated HTTPS link to the operating system.
    fn open(&self, link: &ExternalLink) -> Result<(), ExternalLinkOpenError>;
}

/// A safe failure category returned by an external-link service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalLinkOpenError {
    /// The operating system could not accept the HTTPS handoff.
    Unavailable,
}

impl fmt::Display for ExternalLinkOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("external link handler is unavailable")
    }
}

impl std::error::Error for ExternalLinkOpenError {}

fn validate_authority(authority: &str) -> Result<(), ExternalLinkInputError> {
    if authority.is_empty() || authority.contains('@') {
        return Err(ExternalLinkInputError::InvalidAuthority);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    validate_hostname(host)?;
    if let Some(port) = port {
        let Ok(port) = port.parse::<u16>() else {
            return Err(ExternalLinkInputError::InvalidPort);
        };
        if port == 0 {
            return Err(ExternalLinkInputError::InvalidPort);
        }
    }
    Ok(())
}

fn validate_hostname(host: &str) -> Result<(), ExternalLinkInputError> {
    if host.is_empty() || host.len() > 253 || host.ends_with('.') {
        return Err(ExternalLinkInputError::InvalidHost);
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || !label.starts_with(|character: char| character.is_ascii_alphanumeric())
            || !label.ends_with(|character: char| character.is_ascii_alphanumeric())
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(ExternalLinkInputError::InvalidHost);
        }
    }
    Ok(())
}

/// A safe validation failure before a native link-opening call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalLinkInputError {
    /// The value was empty.
    Empty,
    /// The value exceeded the fixed URL limit.
    TooLong,
    /// The value was not an HTTPS URL.
    UnsupportedScheme,
    /// The URL contained a control, whitespace, non-ASCII byte, or backslash.
    InvalidCharacter,
    /// The authority contained user information or had no host.
    InvalidAuthority,
    /// The hostname was not a bounded DNS-style name.
    InvalidHost,
    /// The optional port was malformed or outside the accepted range.
    InvalidPort,
}

impl fmt::Display for ExternalLinkInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "external link is empty",
            Self::TooLong => "external link exceeds the fixed size limit",
            Self::UnsupportedScheme => "external link must use HTTPS",
            Self::InvalidCharacter => "external link contains unsupported characters",
            Self::InvalidAuthority => "external link authority is invalid",
            Self::InvalidHost => "external link host is invalid",
            Self::InvalidPort => "external link port is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExternalLinkInputError {}

#[cfg(test)]
mod tests {
    use super::{ExternalLink, ExternalLinkInputError, MAX_EXTERNAL_LINK_BYTES};

    #[test]
    fn accepts_bounded_https_urls_without_rewriting_them() {
        let link = ExternalLink::parse("https://docs.anodrel.dev:8443/guide?q=owned#native")
            .expect("fixture URL is valid");
        assert_eq!(
            link.as_str(),
            "https://docs.anodrel.dev:8443/guide?q=owned#native"
        );
    }

    #[test]
    fn rejects_every_non_https_or_shell_like_input_before_native_handoff() {
        for (value, expected) in [
            ("", ExternalLinkInputError::Empty),
            (
                "http://example.com",
                ExternalLinkInputError::UnsupportedScheme,
            ),
            (
                "file:///C:/secret.txt",
                ExternalLinkInputError::UnsupportedScheme,
            ),
            (
                "https://user@example.com",
                ExternalLinkInputError::InvalidAuthority,
            ),
            (
                "https://example.com\\command",
                ExternalLinkInputError::InvalidCharacter,
            ),
            ("https://-bad.example", ExternalLinkInputError::InvalidHost),
            ("https://example.com:0", ExternalLinkInputError::InvalidPort),
        ] {
            assert_eq!(ExternalLink::parse(value), Err(expected));
        }
    }

    #[test]
    fn rejects_values_that_exceed_the_fixed_url_bound() {
        let value = format!("https://{}.example", "a".repeat(MAX_EXTERNAL_LINK_BYTES));
        assert_eq!(
            ExternalLink::parse(value),
            Err(ExternalLinkInputError::TooLong)
        );
    }
}
