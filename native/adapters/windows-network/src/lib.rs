#![deny(unsafe_op_in_unsafe_fn)]

//! Direct, host-only Windows HTTPS text fetches through shared WinHTTP.
//!
//! This adapter owns a host-created exact origin policy and performs only one
//! bounded secure `GET` for a previously validated URL. It supplies no browser
//! state, proxy discovery, cookie, credential, redirect, or native-handle
//! surface. See `docs/NETWORK.md`, `docs/HTTPS_TRANSPORT.md`, and Decision
//! 0084.

use std::fmt;

use anodrel_network::{
    MAX_NETWORK_TEXT_BYTES, NetworkOrigin, NetworkOriginPolicy, NetworkTextResponse,
    NetworkTextService, NetworkTextServiceError, NetworkUrl,
};
use anodrel_windows_http::{WindowsHttpsError, get_https};

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
        let origin = NetworkOrigin::new(url.hostname(), url.port())
            .map_err(|_| NetworkTextServiceError::Unavailable)?;
        let mut bytes = Vec::with_capacity(4 * 1024);
        let status = get_https(
            &origin,
            url.request_target(),
            None,
            MAX_NETWORK_TEXT_BYTES,
            &mut |chunk| {
                bytes.try_reserve(chunk.len()).map_err(|_| ())?;
                bytes.extend_from_slice(chunk);
                Ok(())
            },
        )
        .map_err(map_transport_error)?;
        let body =
            String::from_utf8(bytes).map_err(|_| NetworkTextServiceError::ResponseInvalid)?;
        NetworkTextResponse::new(status, body).map_err(|_| NetworkTextServiceError::ResponseInvalid)
    }
}

fn map_transport_error(error: WindowsHttpsError) -> NetworkTextServiceError {
    match error {
        WindowsHttpsError::RequestInvalid | WindowsHttpsError::Unavailable => {
            NetworkTextServiceError::Unavailable
        }
        WindowsHttpsError::ResponseInvalid
        | WindowsHttpsError::UnexpectedStatus
        | WindowsHttpsError::BodyTooLarge
        | WindowsHttpsError::ConsumerRejected => NetworkTextServiceError::ResponseInvalid,
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
