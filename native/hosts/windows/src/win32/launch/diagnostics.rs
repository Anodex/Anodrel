//! Fixed diagnostic launch paths and startup readings.

use super::super::*;
use anodrel_diagnostics::Event;

/// Builds the diagnostic history displayed by the Startup Lab.
///
/// Its only input is one member of the closed event catalogue, chosen by the
/// caller's preflight. The displayed history therefore still reflects fixed host
/// milestones rather than application text, operating-system errors, paths, or
/// arbitrary caller data.
pub(crate) fn startup_log_book(launch_event: Event) -> LogBook {
    let mut log = LogBook::new();
    // Chronological: the preflight runs alongside the two checks above it and is
    // settled before this surface is authorized to open.
    for event in [
        Event::PackageVerified,
        Event::CoreHealthChecked,
        Event::PipeLoopbackChecked,
        launch_event,
        Event::StartupLabAuthorized,
    ] {
        log.record(event)
            .expect("five fixed startup events fit in the diagnostic log");
    }
    log
}

/// Deliberate fault injection, for proving the containment path end to end.
///
/// A crash reporter that nobody can trigger is one nobody knows still works.
/// The `--crash-report-selftest` route proves the store can write; this proves
/// the part that matters more — that a panic raised inside a real window
/// message is contained, classified, recorded, and shut down cleanly instead of
/// aborting the process.
///
/// Compiled only in a debug build. `start.bat` and every shipped path build in
/// release, so this cannot exist in a binary a user runs.
#[cfg(debug_assertions)]
pub(crate) mod crash_selftest {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Arms the next window paint to panic.
    pub(crate) fn arm() {
        ARMED.store(true, Ordering::Release);
    }

    /// Panics once if armed, disarming itself first.
    ///
    /// Disarming before panicking matters: the host repaints while shutting
    /// down, and a fault that re-armed itself would panic again inside the
    /// cleanup this route exists to observe.
    pub(crate) fn fault_if_armed() {
        if ARMED.swap(false, Ordering::AcqRel) {
            panic!("deliberate fault injected by --crash-selftest-panic");
        }
    }
}

/// Opens the UI Lab and panics inside its first paint, then reports the result.
///
/// Available in debug builds only. What to look for afterwards: the process
/// exits without aborting, and a new record appears in the location named by
/// `docs/CRASH_REPORTS.md` with `site=window-procedure` and `surface=ui-lab` —
/// the surface being the proof that classification ran against a live window
/// rather than falling back to `unknown`.
#[cfg(debug_assertions)]
pub fn run_crash_selftest_panic() -> Result<(), Box<dyn std::error::Error>> {
    crash_selftest::arm();
    super::routes::run_ui_lab()?;
    println!(
        "The injected fault was contained and the host shut down. \
         Check for a record with surface=ui-lab; see docs/CRASH_REPORTS.md."
    );
    Ok(())
}

/// Runs the real startup sequence, prints its readings, and exits.
///
/// The Startup Lab already shows a startup time and a working set in its
/// footer. This is the same measurement in a form a script can keep, so a
/// startup or memory figure can be recorded across builds instead of read off a
/// screenshot.
///
/// # What it measures, and what it does not
///
/// The elapsed time covers everything the host does before a surface could
/// open: package verification, the core health check, the private pipe
/// loopback, and the launch preflight. It stops there. **It does not include
/// creating the window or its first paint**, so it is a floor for cold start
/// and must never be quoted as time-to-first-frame. See `docs/PERFORMANCE.md`.
///
/// No window opens, so the memory readings are the host's own cost with nothing
/// rendered — also a floor, and the honest one to compare against another
/// runtime's idle process.
pub fn print_startup_report(application_id: &str, elapsed: std::time::Duration) {
    let memory = stats::memory_readings();
    // A local tooling format, shaped like the performance lab's report so both
    // can be retained the same way. It is not a protocol and has no reader here.
    println!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.startup.v1\",",
            "\"startupMicroseconds\":{},",
            "\"workingSetBytes\":{},\"privateBytes\":{},",
            "\"applicationId\":\"{}\",",
            "\"scope\":\"host startup checks only; no window creation, no first paint\"}}"
        ),
        elapsed.as_micros(),
        memory.working_set_bytes,
        memory.private_bytes,
        application_id,
    );
}

/// Writes one crash record through the ordinary reporting path, then exits.
///
/// A crash record is only useful if it is actually written on the machine it is
/// meant to help, and the path that writes it is reached exactly once, during a
/// defect, when nobody is watching. This is how an operator confirms the
/// location, permissions, and format without waiting for a real failure.
///
/// It records the same `window-procedure` site a contained panic does, with no
/// window registered, so the surface is `unknown`. It opens no window and
/// prints only whether the record was written — never the location, which is in
/// `docs/CRASH_REPORTS.md` for a person to look up rather than for a process to
/// hand out.
pub fn run_crash_report_selftest() -> Result<(), Box<dyn std::error::Error>> {
    match crash::report(CrashSite::WindowProcedure, CrashSurface::Unknown) {
        Some(sequence) => {
            println!("Wrote crash record {sequence}. See docs/CRASH_REPORTS.md for its location.");
            Ok(())
        }
        None => Err(io::Error::other("the host could not write a crash record").into()),
    }
}
