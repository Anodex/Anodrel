//! Monotonic revisions for one current UI document.

/// The opaque generation of one UI document session state.
///
/// Revision zero means the session has never accepted or cleared a document.
/// Successful replacements and clears advance the revision without wrapping.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct UiDocumentRevision(u64);

impl UiDocumentRevision {
    /// The initial revision for an empty session.
    pub const INITIAL: Self = Self(0);

    /// Returns the revision's monotonically increasing numeric value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
