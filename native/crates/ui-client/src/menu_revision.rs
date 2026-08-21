//! Typed menu revisions carried as canonical decimal protocol strings.

use std::num::NonZeroU64;

use crate::{UiClientError, revision::parse_nonzero_decimal};

/// The nonzero revision of one accepted native session menu.
///
/// Native command numbers remain host-private. This value is only the portable
/// complete-model revision used to validate a returned semantic action.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MenuRevision(NonZeroU64);

impl MenuRevision {
    /// Returns the native integer value of the accepted menu revision.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub(crate) fn parse(value: &str) -> Result<Self, UiClientError> {
        Ok(Self(parse_nonzero_decimal(value)?))
    }
}
