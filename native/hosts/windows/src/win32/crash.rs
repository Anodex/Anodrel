//! Recording a contained panic, on the way down.
//!
//! Everything here runs after a defect has already happened, so nothing here
//! may make things worse. It does not panic, does not block, does not report to
//! a user or an application, and does not stop the shutdown it is part of.
//!
//! See `docs/CRASH_REPORTS.md` and Decision 0065 for what a record may say.

use std::sync::OnceLock;

use anodrel_crash::{CrashRecord, CrashReporter, CrashSite, CrashSurface};
use anodrel_windows_crash::WindowsCrashStore;

use super::{Hwnd, registry};

/// The version written into every record from this host.
const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The store, resolved once.
///
/// Resolved lazily rather than at startup so a host that never panics makes no
/// known-folder call for this, and held in a `OnceLock` so the resolution
/// happens at most once even if it fails. Building it performs no I/O: it is a
/// path lookup and some joins, which is work worth doing even here.
static STORE: OnceLock<Option<WindowsCrashStore>> = OnceLock::new();

fn store() -> Option<&'static WindowsCrashStore> {
    STORE
        .get_or_init(|| {
            anodrel_windows_paths::host_directories()
                .ok()
                .map(|directories| WindowsCrashStore::new(&directories))
        })
        .as_ref()
}

/// Records that a panic was contained while serving `window`.
///
/// Returns the sequence the store assigned, for tests and for the self-test
/// route. Callers on the shutdown path ignore it: there is nothing useful to do
/// with a failure at that point, and trying to do something would be the
/// mistake this whole module exists to avoid.
pub(super) fn report_contained_panic(window: Hwnd) -> Option<u64> {
    report(CrashSite::WindowProcedure, registry::crash_surface(window))
}

/// Records one crash, containing any failure inside this call.
///
/// The reporting path itself is wrapped: a store that panicked while recording
/// a panic would abort the process through the same `extern "system"` boundary
/// the containment exists to protect. Belt and braces, at a moment where the
/// cost of being wrong is the thing being guarded against.
pub(super) fn report(site: CrashSite, surface: CrashSurface) -> Option<u64> {
    super::contain_panic(|| {
        let record = CrashRecord::new(site, surface, HOST_VERSION);
        store()?.report(&record).ok()
    })
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::HOST_VERSION;

    #[test]
    fn the_host_version_is_writable_as_a_record_field() {
        // anodrel-crash refuses a version the line format cannot carry. That
        // check is only useful if this host's own version passes it, and a
        // version is the one record field that is not a catalogue value.
        let record = anodrel_crash::CrashRecord::new(
            anodrel_crash::CrashSite::WindowProcedure,
            anodrel_crash::CrashSurface::Unknown,
            HOST_VERSION,
        );
        assert!(anodrel_crash::serialize(&record, 1).is_ok());
    }
}
