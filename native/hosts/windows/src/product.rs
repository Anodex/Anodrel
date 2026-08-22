//! The host-only activation route for one verified Windows product session.
//!
//! This module owns thread placement and nothing else. It starts the
//! product-session coordinator on a worker, because that work performs blocking
//! machine-policy, locked digest, Authenticode, and process-creation calls that
//! must never run on the Win32 UI thread. Only after the worker reports success
//! does the calling thread open the authenticated native window with that one
//! session's grouped UI resources.
//!
//! It is not reachable from an application, a package, a protocol message, or
//! rendered content. See `docs/PRODUCT_FIXTURE.md` and `docs/PRODUCT_SESSIONS.md`.

use std::{error::Error, io, thread};

use anodrel_application::is_valid_application_id;
use anodrel_diagnostics::Event;
use anodrel_windows_launch::verify_registered_application;
use anodrel_windows_product_session::{
    ProductSessionError, RunningProductSession, start_registered_product_session,
};

const HOST_NAME: &str = "anodrel-windows-host";
/// Named in full so the window can never be mistaken for a product session.
const WINDOW_TITLE: &str = "Anodrel Development Product Fixture";

/// Runs one verified product session and its native window to completion.
///
/// `application_id` selects which already-provisioned machine record to read. It
/// cannot supply a record, a package, an executable, a capability, or a child
/// argument; every one of those still comes from machine policy.
pub fn run(application_id: &str) -> Result<(), Box<dyn Error>> {
    if !is_valid_application_id(application_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the product session application identity is invalid",
        )
        .into());
    }

    let session = start_off_the_ui_thread(application_id)?;
    // The window runs on this thread while `session` stays alive. Dropping the
    // session on any error path below still requests full shutdown, so a child
    // cannot be orphaned by an early return.
    let ui = session.ui();
    let window_result = crate::win32::run_authenticated_ui_session(
        WINDOW_TITLE,
        ui.document_mailbox(),
        ui.input_mailbox(),
        ui.close_signal(),
        ui.file_dialog_mailbox(),
        ui.file_text_service(),
        ui.notification_mailbox(),
        ui.menu_mailbox(),
        ui.window_title_mailbox(),
        ui.window_state_mailbox(),
        ui.window_focus_mailbox(),
        ui.window_fullscreen_mailbox(),
        ui.window_size_mailbox(),
        // From the identity that matched the machine-validated installed
        // record, never from the child. This is the half of a caption the
        // application cannot influence.
        ui.display_name(),
        ui.field_mailbox(),
    );
    let finish_result = session.finish();
    window_result?;
    finish_result?;
    Ok(())
}

/// One launch preflight running on its own worker thread.
///
/// The preflight is the most expensive thing the Startup Lab does before its
/// window exists: on a provisioned machine it hashes the whole executable
/// through a lock and asks Windows to evaluate an Authenticode chain, which can
/// reach revocation infrastructure. Running it beside the core health check and
/// the private pipe loopback removes almost all of that from the critical path.
///
/// The answer is still required *before* the window is created. Every document
/// that describes the launch tile promises its state is resolved before the
/// surface opens, and keeping that promise is what lets drawing and hit-testing
/// share one immutable value instead of a tile that changes under the pointer.
pub struct FixturePreflight(Option<thread::JoinHandle<bool>>);

impl FixturePreflight {
    /// Starts the verification-only preflight for the development fixture.
    ///
    /// Call this as early as the host knows it owns the surface, then call
    /// [`Self::finish`] immediately before window creation.
    #[must_use]
    pub fn begin() -> Self {
        Self(
            thread::Builder::new()
                .name("anodrel-product-preflight".to_owned())
                .spawn(|| is_launchable(crate::win32::FIXTURE_APPLICATION_ID))
                .ok(),
        )
    }

    /// Waits for the preflight and reports what the surface may offer.
    ///
    /// A worker that could not be started, or that stopped unexpectedly, reports
    /// [`PreflightOutcome::Unavailable`]: an unavailable check never widens what
    /// the surface offers.
    #[must_use]
    pub fn finish(self) -> PreflightOutcome {
        match self.0.map(thread::JoinHandle::join) {
            Some(Ok(true)) => PreflightOutcome::Launchable,
            Some(Ok(false)) => PreflightOutcome::NotLaunchable,
            None | Some(Err(_)) => PreflightOutcome::Unavailable,
        }
    }
}

/// What one completed or abandoned launch preflight tells the host.
///
/// The distinction between "ran and declined" and "never ran" is not visible on
/// the surface — both leave the tile planned — so it is the one thing worth
/// recording in the diagnostic log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreflightOutcome {
    /// The record and signature validate right now.
    Launchable,
    /// The check ran and this machine cannot launch the fixture.
    NotLaunchable,
    /// The check could not run, so nothing may be offered.
    Unavailable,
}

impl PreflightOutcome {
    /// Whether a launch action may exist on the surface at all.
    #[must_use]
    pub const fn allows_launch(self) -> bool {
        matches!(self, Self::Launchable)
    }

    /// The closed diagnostic event describing the host's own execution.
    ///
    /// The event states whether this host completed its check, never what the
    /// check concluded, so the ledger cannot report whether another application
    /// is provisioned on this machine.
    #[must_use]
    pub const fn event(self) -> Event {
        match self {
            Self::Launchable | Self::NotLaunchable => Event::LaunchPreflightCompleted,
            Self::Unavailable => Event::LaunchPreflightUnavailable,
        }
    }
}

impl std::fmt::Debug for FixturePreflight {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("FixturePreflight(..)")
    }
}

/// Reports whether a registered application is currently launchable.
///
/// This runs the launch verification sequence without creating a process, so a
/// host surface can decide whether to offer a launch action at all. An invalid
/// identity, an absent record, a changed executable, or a rejected signature all
/// answer `false` with no detail.
///
/// It blocks on machine work, so a UI thread must reach it through
/// [`FixturePreflight`] rather than calling it directly. The answer describes
/// this moment only; every launch re-runs the full sequence.
#[must_use]
fn is_launchable(application_id: &str) -> bool {
    is_valid_application_id(application_id) && verify_registered_application(application_id).is_ok()
}

/// Starts the coordinator on a worker and waits for its result.
fn start_off_the_ui_thread(
    application_id: &str,
) -> Result<RunningProductSession, ProductSessionError> {
    let application_id = application_id.to_owned();
    let session_id = format!("product-{}", std::process::id());
    let worker = thread::Builder::new()
        .name("anodrel-product-start".to_owned())
        .spawn(move || start_registered_product_session(&application_id, HOST_NAME, session_id))
        .map_err(ProductSessionError::WorkerStart)?;

    worker.join().map_err(|_| {
        ProductSessionError::WorkerStart(io::Error::other(
            "the product session starter stopped unexpectedly",
        ))
    })?
}

#[cfg(test)]
mod tests {
    use super::{FixturePreflight, PreflightOutcome, is_launchable, run};

    #[test]
    fn an_invalid_identity_never_reaches_machine_policy_or_a_window() {
        let error = run("org.anodrel/escape").expect_err("an invalid identity is rejected");
        assert!(error.to_string().contains("identity is invalid"));
        assert!(!is_launchable("org.anodrel/escape"));
    }

    #[test]
    fn an_unprovisioned_application_is_not_launchable() {
        // The Startup Lab reads exactly this value to decide whether its launch
        // tile may be linked, so it must fail closed on a clean machine.
        assert!(!is_launchable(
            "org.anodrel.product-route-unprovisioned-test"
        ));
    }

    #[test]
    fn a_backgrounded_preflight_answers_the_same_way_as_a_direct_check() {
        // Moving the preflight off the startup path must not change what the
        // surface is allowed to offer.
        assert_eq!(
            FixturePreflight::begin().finish().allows_launch(),
            is_launchable(crate::win32::FIXTURE_APPLICATION_ID)
        );
    }

    #[test]
    fn a_preflight_that_never_ran_reports_no_launch_action() {
        let outcome = FixturePreflight(None).finish();
        assert_eq!(outcome, PreflightOutcome::Unavailable);
        assert!(!outcome.allows_launch());
    }

    #[test]
    fn only_a_launchable_outcome_permits_a_launch_action() {
        assert!(PreflightOutcome::Launchable.allows_launch());
        assert!(!PreflightOutcome::NotLaunchable.allows_launch());
        assert!(!PreflightOutcome::Unavailable.allows_launch());
    }

    #[test]
    fn the_diagnostic_event_separates_a_declined_check_from_an_absent_one() {
        // Both leave the tile planned, so the log is the only place an operator
        // can tell them apart. Neither event reports the answer itself.
        assert_eq!(
            PreflightOutcome::Launchable.event(),
            PreflightOutcome::NotLaunchable.event()
        );
        assert_ne!(
            PreflightOutcome::NotLaunchable.event(),
            PreflightOutcome::Unavailable.event()
        );
    }

    #[test]
    fn a_preflight_handle_never_reveals_its_result_in_debug_output() {
        assert_eq!(
            format!("{:?}", FixturePreflight(None)),
            "FixturePreflight(..)"
        );
    }
}
