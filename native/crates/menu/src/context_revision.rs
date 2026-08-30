use std::fmt;

/// A nonzero monotonic revision of one complete context-menu model.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ContextMenuRevision(u64);

impl ContextMenuRevision {
    /// The empty session's initial revision.
    pub const INITIAL: Self = Self(0);

    /// Returns the next revision, or `None` instead of wrapping.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the canonical decimal value for a protocol adapter.
    #[must_use]
    pub fn as_decimal(self) -> String {
        self.0.to_string()
    }

    /// Returns the underlying revision for portable tests and ordering.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Default for ContextMenuRevision {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl fmt::Display for ContextMenuRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
