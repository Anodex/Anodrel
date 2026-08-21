//! Strict portable parsing for one HTTPS text-fetch URL.

use std::fmt;

/// Maximum bytes in one HTTPS text-fetch URL.
pub const MAX_NETWORK_URL_BYTES: usize = 2_048;

/// One validated HTTPS URL for a host-authorized text request.
///
/// The original value remains available for diagnostics-free request handling,
/// while the hostname is canonicalized only for exact host-policy comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkUrl {
    value: String,
    hostname: String,
    port: u16,
    request_target: String,
}

impl NetworkUrl {
    /// Parses one strict, bounded HTTPS URL with a DNS-style hostname.
    pub fn parse(value: impl Into<String>) -> Result<Self, NetworkUrlError> {
        let value = value.into();
        if value.is_empty() {
            return Err(NetworkUrlError::Empty);
        }
        if value.len() > MAX_NETWORK_URL_BYTES {
            return Err(NetworkUrlError::TooLong);
        }
        if value.contains('#') {
            return Err(NetworkUrlError::FragmentNotAllowed);
        }
        if !value.is_ascii()
            || value
                .bytes()
                .any(|byte| !is_allowed_url_byte(byte) || byte == b'\\')
        {
            return Err(NetworkUrlError::InvalidCharacter);
        }
        let Some(remainder) = value.strip_prefix("https://") else {
            return Err(NetworkUrlError::UnsupportedScheme);
        };
        let authority_end = remainder.find(['/', '?']).unwrap_or(remainder.len());
        let authority = &remainder[..authority_end];
        let (hostname, port) = parse_authority(authority)?;
        let suffix = &remainder[authority_end..];
        let request_target = request_target(suffix)?;
        Ok(Self {
            value,
            hostname,
            port,
            request_target,
        })
    }

    /// Returns the exact validated URL without normalization or rewriting.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Returns the canonical lowercase DNS hostname used for policy matching.
    #[must_use]
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the effective HTTPS port, using 443 when the URL omitted it.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Returns the validated path and query that a native adapter may request.
    ///
    /// A URL with no explicit path has the normal `/` request target; a
    /// query-only URL is represented as `/?query`.
    #[must_use]
    pub fn request_target(&self) -> &str {
        &self.request_target
    }
}

pub(crate) fn validate_hostname(value: &str) -> Result<String, NetworkUrlError> {
    if value.is_empty() || value.len() > 253 || value.ends_with('.') || is_ipv4_literal(value) {
        return Err(NetworkUrlError::InvalidHost);
    }
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || !label.starts_with(|character: char| character.is_ascii_alphanumeric())
            || !label.ends_with(|character: char| character.is_ascii_alphanumeric())
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(NetworkUrlError::InvalidHost);
        }
    }
    Ok(value.to_ascii_lowercase())
}

fn parse_authority(authority: &str) -> Result<(String, u16), NetworkUrlError> {
    if authority.is_empty() || authority.contains('@') {
        return Err(NetworkUrlError::InvalidAuthority);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, value)) => (host, Some(value)),
        None => (authority, None),
    };
    let hostname = validate_hostname(host)?;
    let port = match port {
        Some(value) => value
            .parse::<u16>()
            .ok()
            .filter(|value| *value != 0)
            .ok_or(NetworkUrlError::InvalidPort)?,
        None => 443,
    };
    Ok((hostname, port))
}

fn request_target(suffix: &str) -> Result<String, NetworkUrlError> {
    let target = match suffix {
        "" => "/".to_owned(),
        value if value.starts_with('/') => value.to_owned(),
        value if value.starts_with('?') => format!("/{value}"),
        _ => return Err(NetworkUrlError::InvalidCharacter),
    };
    if malformed_percent_escape(&target) {
        return Err(NetworkUrlError::MalformedPercentEscape);
    }
    Ok(target)
}

fn malformed_percent_escape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| {
        *byte == b'%'
            && (index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit())
    })
}

fn is_ipv4_literal(value: &str) -> bool {
    let labels = value.split('.').collect::<Vec<_>>();
    labels.len() == 4
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.bytes().all(|byte| byte.is_ascii_digit())
                && label.parse::<u8>().is_ok()
        })
}

fn is_allowed_url_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b':'
                | b'/'
                | b'?'
                | b'@'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'%'
        )
}

/// A safe validation failure before any native network call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkUrlError {
    /// The URL was empty.
    Empty,
    /// The URL exceeded the fixed protocol limit.
    TooLong,
    /// The URL did not use exactly the `https` scheme.
    UnsupportedScheme,
    /// The URL contained an unsupported byte, whitespace, control, or backslash.
    InvalidCharacter,
    /// The URL contained a fragment, which the text-fetch contract excludes.
    FragmentNotAllowed,
    /// The authority was missing or included user information.
    InvalidAuthority,
    /// The hostname was not a bounded DNS-style name or was an IPv4 literal.
    InvalidHost,
    /// The optional port was malformed, zero, or larger than 65,535.
    InvalidPort,
    /// A percent escape was incomplete or did not contain two hexadecimal bytes.
    MalformedPercentEscape,
}

impl fmt::Display for NetworkUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "network URL is empty",
            Self::TooLong => "network URL exceeds the fixed size limit",
            Self::UnsupportedScheme => "network URL must use HTTPS",
            Self::InvalidCharacter => "network URL contains unsupported characters",
            Self::FragmentNotAllowed => "network URL must not contain a fragment",
            Self::InvalidAuthority => "network URL authority is invalid",
            Self::InvalidHost => "network URL host is invalid",
            Self::InvalidPort => "network URL port is invalid",
            Self::MalformedPercentEscape => "network URL contains a malformed percent escape",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NetworkUrlError {}

#[cfg(test)]
mod tests {
    use super::{MAX_NETWORK_URL_BYTES, NetworkUrl, NetworkUrlError};

    #[test]
    fn parses_an_https_request_without_rewriting_the_value() {
        let url = NetworkUrl::parse("https://Api.Example.test:8443/v1/status?format=text%2Fplain")
            .expect("fixture URL is valid");
        assert_eq!(
            url.as_str(),
            "https://Api.Example.test:8443/v1/status?format=text%2Fplain"
        );
        assert_eq!(url.hostname(), "api.example.test");
        assert_eq!(url.port(), 8443);
        assert_eq!(url.request_target(), "/v1/status?format=text%2Fplain");
    }

    #[test]
    fn supplies_the_normal_root_target_for_pathless_urls() {
        assert_eq!(
            NetworkUrl::parse("https://api.example.test")
                .expect("pathless URL is valid")
                .request_target(),
            "/"
        );
        assert_eq!(
            NetworkUrl::parse("https://api.example.test?format=text")
                .expect("query-only URL is valid")
                .request_target(),
            "/?format=text"
        );
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_values_before_native_networking() {
        for (value, expected) in [
            ("", NetworkUrlError::Empty),
            (
                "http://api.example.test",
                NetworkUrlError::UnsupportedScheme,
            ),
            (
                "https://user@api.example.test",
                NetworkUrlError::InvalidAuthority,
            ),
            ("https://127.0.0.1/status", NetworkUrlError::InvalidHost),
            (
                "https://api.example.test/#fragment",
                NetworkUrlError::FragmentNotAllowed,
            ),
            (
                "https://api.example.test/with space",
                NetworkUrlError::InvalidCharacter,
            ),
            (
                "https://api.example.test\\command",
                NetworkUrlError::InvalidCharacter,
            ),
            (
                "https://api.example.test/%1",
                NetworkUrlError::MalformedPercentEscape,
            ),
            (
                "https://api.example.test:0/status",
                NetworkUrlError::InvalidPort,
            ),
        ] {
            assert_eq!(NetworkUrl::parse(value), Err(expected));
        }
    }

    #[test]
    fn rejects_values_over_the_fixed_url_limit() {
        let value = format!("https://{}.example.test", "a".repeat(MAX_NETWORK_URL_BYTES));
        assert_eq!(NetworkUrl::parse(value), Err(NetworkUrlError::TooLong));
    }
}
