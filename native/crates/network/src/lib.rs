//! Bounded, host-authorized HTTPS text-fetch values.
//!
//! This portable crate owns URL validation, exact origin matching, and the
//! text-only service seam. It has no socket, proxy, TLS, or operating-system
//! authority. Native adapters receive only a previously validated URL and a
//! host-created origin policy. See `docs/NETWORK.md` and Decision 0084.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod origin;
mod response;
mod service;
mod url;

pub use origin::{
    MAX_NETWORK_ORIGINS, NetworkOrigin, NetworkOriginError, NetworkOriginPolicy,
    NetworkOriginPolicyError,
};
pub use response::{MAX_NETWORK_TEXT_BYTES, NetworkTextResponse, NetworkTextResponseError};
pub use service::{NetworkTextService, NetworkTextServiceError};
pub use url::{MAX_NETWORK_URL_BYTES, NetworkUrl, NetworkUrlError};
