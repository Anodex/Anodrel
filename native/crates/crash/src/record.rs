//! The closed catalogues and the record built from them.

/// Format tag written into every record.
///
/// A reader that does not recognise this must not guess at the fields. Adding a
/// field, a clock, or any caller-supplied text changes it.
pub const FORMAT: &str = "anodrel.crash.v1";

/// Where containment happened.
///
/// A closed catalogue, not a description. Adding a variant is additive; see the
/// compatibility rule in `docs/CRASH_REPORTS.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashSite {
    /// A panic escaped one window-message dispatch and was contained there.
    WindowProcedure,
}

impl CrashSite {
    /// Every site, for exhaustive tests.
    pub const ALL: [Self; 1] = [Self::WindowProcedure];

    /// Returns the stable label written into a record.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WindowProcedure => "window-procedure",
        }
    }
}

/// What kind of window was being served when containment happened.
///
/// This is the host's own view classification, which is why it can be a closed
/// catalogue at all. It names a kind of surface, never a title, document,
/// application identity, or handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrashSurface {
    /// The branded startup surface.
    StartupLab,
    /// A host-composed document window.
    Document,
    /// The owned UI foundation lab.
    UiLab,
    /// A development UI session view.
    UiSession,
    /// No view was registered for the window.
    ///
    /// Distinct from a missing field: a panic before or after a view is
    /// registered is a different defect from one while serving a surface.
    Unknown,
}

impl CrashSurface {
    /// Every surface, for exhaustive tests.
    pub const ALL: [Self; 5] = [
        Self::StartupLab,
        Self::Document,
        Self::UiLab,
        Self::UiSession,
        Self::Unknown,
    ];

    /// Returns the stable label written into a record.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StartupLab => "startup-lab",
            Self::Document => "document",
            Self::UiLab => "ui-lab",
            Self::UiSession => "ui-session",
            Self::Unknown => "unknown",
        }
    }
}

/// One complete crash record.
///
/// Every field is a closed catalogue value, a compile-time constant, or a
/// counter. There is no constructor taking text, so a panic payload cannot
/// reach a record even by mistake.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrashRecord {
    site: CrashSite,
    surface: CrashSurface,
    host_version: &'static str,
    sequence: u64,
}

impl CrashRecord {
    /// Builds a record.
    ///
    /// `host_version` is the host crate's own `CARGO_PKG_VERSION`. It is
    /// `&'static str` rather than `String` so it can only come from a constant.
    #[must_use]
    pub const fn new(
        site: CrashSite,
        surface: CrashSurface,
        host_version: &'static str,
        sequence: u64,
    ) -> Self {
        Self {
            site,
            surface,
            host_version,
            sequence,
        }
    }

    /// Returns where containment happened.
    #[must_use]
    pub const fn site(&self) -> CrashSite {
        self.site
    }

    /// Returns the kind of surface being served.
    #[must_use]
    pub const fn surface(&self) -> CrashSurface {
        self.surface
    }

    /// Returns the host version this record was written by.
    #[must_use]
    pub const fn host_version(&self) -> &'static str {
        self.host_version
    }

    /// Returns the process-local order of this record, starting at 1.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

#[cfg(test)]
mod tests {
    use super::{CrashRecord, CrashSite, CrashSurface};

    #[test]
    fn every_catalogue_label_is_distinct_and_wire_safe() {
        // Labels go into a strict `field=value` line format, so a label
        // carrying a separator or newline would produce a record that parses
        // as something other than what was written.
        let mut labels: Vec<&str> = CrashSite::ALL
            .iter()
            .map(|site| site.label())
            .chain(CrashSurface::ALL.iter().map(|surface| surface.label()))
            .collect();
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "two catalogue labels collide");

        for label in labels {
            assert!(!label.is_empty());
            assert!(
                label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-'),
                "{label:?} is not a plain lowercase ASCII label"
            );
        }
    }

    #[test]
    fn a_record_reports_exactly_what_it_was_built_from() {
        let record = CrashRecord::new(
            CrashSite::WindowProcedure,
            CrashSurface::StartupLab,
            "1.2.3",
            7,
        );
        assert_eq!(record.site(), CrashSite::WindowProcedure);
        assert_eq!(record.surface(), CrashSurface::StartupLab);
        assert_eq!(record.host_version(), "1.2.3");
        assert_eq!(record.sequence(), 7);
    }
}
