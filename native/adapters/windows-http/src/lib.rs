#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

//! Internal direct WinHTTP streaming for caller-authorized HTTPS `GET` calls.
//!
//! This adapter has no application protocol surface. Its callers supply a
//! previously validated exact origin and request target, retain their own
//! policy, and consume each bounded body chunk without this adapter retaining a
//! response body. See `docs/HTTPS_TRANSPORT.md` and Decision 0166.

mod error;
mod raw;

use anodrel_network::NetworkOrigin;

pub use error::WindowsHttpsError;

/// Streams one caller-authorized HTTPS response through a bounded consumer.
///
/// `request_target` must be one ASCII absolute HTTP request target without an
/// embedded NUL. `expected_status` either permits every representable HTTP
/// status or requires one exact status before body consumption. The consumer
/// receives each chunk in order and can reject it without retaining it.
pub fn get_https(
    origin: &NetworkOrigin,
    request_target: &str,
    expected_status: Option<u16>,
    maximum_body_bytes: usize,
    consumer: &mut dyn FnMut(&[u8]) -> Result<(), ()>,
) -> Result<u16, WindowsHttpsError> {
    if !is_valid_request_target(request_target)
        || maximum_body_bytes == 0
        || u32::try_from(maximum_body_bytes).is_err()
        || expected_status.is_some_and(|status| !(100..=599).contains(&status))
    {
        return Err(WindowsHttpsError::RequestInvalid);
    }
    raw::get(
        origin,
        request_target,
        expected_status,
        maximum_body_bytes,
        consumer,
    )
}

fn is_valid_request_target(value: &str) -> bool {
    (1..=2_048).contains(&value.len())
        && value.starts_with('/')
        && value.is_ascii()
        && !value.as_bytes().contains(&0)
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
}

#[cfg(test)]
mod tests {
    use anodrel_network::NetworkOrigin;

    use super::{WindowsHttpsError, get_https, is_valid_request_target};

    #[test]
    fn request_targets_reject_control_and_non_absolute_values_before_networking() {
        for invalid in ["", "status", "\\status", "/with\u{7f}control", "/with\0nul"] {
            assert!(!is_valid_request_target(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn invalid_transfer_bounds_fail_before_a_native_request() {
        let origin = NetworkOrigin::new("updates.example.test", 443).expect("origin is valid");
        assert_eq!(
            get_https(&origin, "/installer.exe", Some(200), 0, &mut |_| Ok(())),
            Err(WindowsHttpsError::RequestInvalid)
        );
    }
}
