#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows HTTPS text fetches through WinHTTP.
//!
//! This adapter owns a host-created exact origin policy and performs only one
//! bounded secure `GET` for a previously validated URL. It supplies no browser
//! state, proxy discovery, cookie, credential, redirect, or native-handle
//! surface. See `docs/NETWORK.md` and Decision 0084.

mod raw;

use std::fmt;

use anodrel_network::{
    NetworkOriginPolicy, NetworkTextResponse, NetworkTextService, NetworkTextServiceError,
    NetworkUrl,
};

/// One direct WinHTTP text-fetch service with a host-created exact-origin policy.
pub struct WindowsNetworkTextService {
    origins: NetworkOriginPolicy,
}

impl WindowsNetworkTextService {
    /// Creates a direct Windows network service for one fixed origin policy.
    ///
    /// The policy must come from host configuration before authentication. An
    /// application request cannot inspect, add, remove, or select its origins.
    #[must_use]
    pub fn new(origins: NetworkOriginPolicy) -> Self {
        Self { origins }
    }
}

impl NetworkTextService for WindowsNetworkTextService {
    fn fetch_text(&self, url: &NetworkUrl) -> Result<NetworkTextResponse, NetworkTextServiceError> {
        if !self.origins.allows(url) {
            return Err(NetworkTextServiceError::Unavailable);
        }
        raw::fetch_text(url).map_err(|error| match error {
            raw::WindowsNetworkError::Unavailable => NetworkTextServiceError::Unavailable,
            raw::WindowsNetworkError::ResponseInvalid => NetworkTextServiceError::ResponseInvalid,
        })
    }
}

impl fmt::Debug for WindowsNetworkTextService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Origin configuration is host policy and must not become incidental
        // diagnostic output from a trait-object formatter.
        formatter.write_str("WindowsNetworkTextService(..)")
    }
}

#[cfg(test)]
mod tests {
    use anodrel_network::{NetworkOrigin, NetworkOriginPolicy, NetworkTextService, NetworkUrl};

    use super::WindowsNetworkTextService;

    #[test]
    fn origin_rejection_finishes_before_any_native_request() {
        let service = WindowsNetworkTextService::new(
            NetworkOriginPolicy::new(vec![
                NetworkOrigin::new("api.example.test", 443).expect("fixture origin is valid"),
            ])
            .expect("fixture policy is valid"),
        );
        let rejected =
            NetworkUrl::parse("https://other.example.test/status").expect("fixture URL is valid");
        assert!(service.fetch_text(&rejected).is_err());
    }

    #[test]
    fn debug_output_does_not_reveal_host_origin_policy() {
        let service = WindowsNetworkTextService::new(
            NetworkOriginPolicy::new(vec![
                NetworkOrigin::new("private-policy.example.test", 443)
                    .expect("fixture origin is valid"),
            ])
            .expect("fixture policy is valid"),
        );
        assert_eq!(format!("{service:?}"), "WindowsNetworkTextService(..)");
    }
}
