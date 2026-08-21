//! Host-created exact HTTPS origin policy values.

use std::fmt;

use crate::{NetworkUrl, NetworkUrlError, url::validate_hostname};

/// Maximum exact origins in the first text-fetch service policy.
pub const MAX_NETWORK_ORIGINS: usize = 8;

/// One canonical HTTPS DNS hostname and effective port selected by a host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkOrigin {
    hostname: String,
    port: u16,
}

impl NetworkOrigin {
    /// Creates one host-selected HTTPS origin from a DNS hostname and port.
    pub fn new(hostname: impl Into<String>, port: u16) -> Result<Self, NetworkOriginError> {
        let hostname =
            validate_hostname(&hostname.into()).map_err(NetworkOriginError::InvalidHost)?;
        if port == 0 {
            return Err(NetworkOriginError::InvalidPort);
        }
        Ok(Self { hostname, port })
    }

    /// Returns the canonical lowercase DNS hostname.
    #[must_use]
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the explicit effective HTTPS port.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Returns whether this origin exactly matches a validated request URL.
    #[must_use]
    pub fn allows(&self, url: &NetworkUrl) -> bool {
        self.hostname == url.hostname() && self.port == url.port()
    }
}

/// A fixed non-empty set of host-selected text-fetch origins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkOriginPolicy {
    origins: Vec<NetworkOrigin>,
}

impl NetworkOriginPolicy {
    /// Creates one exact origin policy with between one and eight unique origins.
    pub fn new(origins: Vec<NetworkOrigin>) -> Result<Self, NetworkOriginPolicyError> {
        if origins.is_empty() {
            return Err(NetworkOriginPolicyError::Empty);
        }
        if origins.len() > MAX_NETWORK_ORIGINS {
            return Err(NetworkOriginPolicyError::TooMany);
        }
        if origins
            .iter()
            .enumerate()
            .any(|(index, origin)| origins[..index].contains(origin))
        {
            return Err(NetworkOriginPolicyError::Duplicate);
        }
        Ok(Self { origins })
    }

    /// Returns whether a URL's canonical hostname and effective port match.
    #[must_use]
    pub fn allows(&self, url: &NetworkUrl) -> bool {
        self.origins.iter().any(|origin| origin.allows(url))
    }

    /// Returns the host-created origins without exposing mutable policy state.
    #[must_use]
    pub fn origins(&self) -> &[NetworkOrigin] {
        &self.origins
    }
}

/// A host-side error while constructing an exact origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkOriginError {
    /// The hostname did not meet the URL hostname grammar.
    InvalidHost(NetworkUrlError),
    /// The host supplied port zero.
    InvalidPort,
}

impl fmt::Display for NetworkOriginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHost(_) => formatter.write_str("network origin host is invalid"),
            Self::InvalidPort => formatter.write_str("network origin port is invalid"),
        }
    }
}

impl std::error::Error for NetworkOriginError {}

/// A host-side error while building the exact origin set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkOriginPolicyError {
    /// The policy had no origins.
    Empty,
    /// The policy exceeded the first service's fixed origin limit.
    TooMany,
    /// The policy repeated one canonical hostname and effective port.
    Duplicate,
}

impl fmt::Display for NetworkOriginPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "network origin policy must not be empty",
            Self::TooMany => "network origin policy exceeds the fixed origin limit",
            Self::Duplicate => "network origin policy contains a duplicate origin",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NetworkOriginPolicyError {}

#[cfg(test)]
mod tests {
    use crate::{NetworkOrigin, NetworkOriginPolicy, NetworkOriginPolicyError, NetworkUrl};

    #[test]
    fn exact_policy_matching_uses_canonical_host_and_effective_port() {
        let policy = NetworkOriginPolicy::new(vec![
            NetworkOrigin::new("Api.Example.test", 443).expect("origin is valid"),
            NetworkOrigin::new("status.example.test", 8443).expect("origin is valid"),
        ])
        .expect("policy is valid");
        assert!(
            policy.allows(&NetworkUrl::parse("https://api.example.test/v1").expect("URL is valid"))
        );
        assert!(policy.allows(
            &NetworkUrl::parse("https://status.example.test:8443/health").expect("URL is valid")
        ));
        assert!(!policy.allows(
            &NetworkUrl::parse("https://status.example.test/health").expect("URL is valid")
        ));
        assert!(!policy.allows(
            &NetworkUrl::parse("https://other.example.test/health").expect("URL is valid")
        ));
    }

    #[test]
    fn policy_refuses_empty_duplicate_or_oversized_origin_sets() {
        assert_eq!(
            NetworkOriginPolicy::new(vec![]),
            Err(NetworkOriginPolicyError::Empty)
        );
        let origin = NetworkOrigin::new("api.example.test", 443).expect("origin is valid");
        assert_eq!(
            NetworkOriginPolicy::new(vec![origin.clone(), origin]),
            Err(NetworkOriginPolicyError::Duplicate)
        );
        let origins = (0..9)
            .map(|index| {
                NetworkOrigin::new(format!("api-{index}.example.test"), 443)
                    .expect("fixture origin is valid")
            })
            .collect();
        assert_eq!(
            NetworkOriginPolicy::new(origins),
            Err(NetworkOriginPolicyError::TooMany)
        );
    }
}
