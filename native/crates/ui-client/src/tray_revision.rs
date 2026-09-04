//! Typed tray revisions carried as canonical decimal protocol strings.

use std::num::NonZeroU64;

use crate::{UiClientError, revision::parse_nonzero_decimal};

/// The nonzero revision of one accepted native tray model.
///
/// The notification-area icon, popup placement, and private command IDs remain
/// host-owned. This value only identifies the complete semantic model that
/// produced an action.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TrayRevision(NonZeroU64);

impl TrayRevision {
    /// Returns the native integer value of the accepted tray revision.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0.get()
    }

    pub(crate) fn parse(value: &str) -> Result<Self, UiClientError> {
        Ok(Self(parse_nonzero_decimal(value)?))
    }
}
