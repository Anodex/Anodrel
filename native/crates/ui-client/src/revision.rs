//! Typed document revisions carried as canonical decimal protocol strings.

use std::num::NonZeroU64;

use crate::UiClientError;

/// The nonzero revision of one accepted UI document.
///
/// Wire values are decimal strings so JavaScript clients do not lose precision.
/// This native facade parses only canonical unsigned decimal strings and keeps
/// the result as an integer; no floating-point conversion is involved.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DocumentRevision(NonZeroU64);

impl DocumentRevision {
    /// Returns the native integer value of the accepted revision.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub(crate) fn parse(value: &str) -> Result<Self, UiClientError> {
        Ok(Self(parse_nonzero_decimal(value)?))
    }
}

pub(crate) fn parse_nonzero_decimal(value: &str) -> Result<NonZeroU64, UiClientError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(UiClientError::ResponseInvalid);
    }
    value
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new)
        .filter(|revision| revision.get().to_string() == value)
        .ok_or(UiClientError::ResponseInvalid)
}
