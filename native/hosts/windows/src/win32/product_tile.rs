//! Host-owned handoff between the Startup Lab's launch tile and one product
//! session.
//!
//! Starting a product session performs blocking machine-policy, locked digest,
//! Authenticode, and process-creation work. None of it may run on the Win32 UI
//! thread while a message loop is pumping, so a click only starts a worker. The
//! worker posts one private window message back, and the UI thread creates the
//! product window from the session's own grouped resources.
//!
//! Exactly one product session may exist at a time. Nothing here is reachable
//! from an application, a package, a protocol message, or rendered content.

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, AtomicIsize, Ordering},
};

use anodrel_windows_product_session::{RunningProductSession, start_registered_product_session};

use super::Hwnd;

const HOST_NAME: &str = "anodrel-windows-host";

/// The one machine record the Startup Lab tile may activate.
///
/// It is a compile-time constant: the surface cannot be pointed at another
/// application's policy key by a click, a package value, or rendered content.
pub const FIXTURE_APPLICATION_ID: &str = "org.anodrel.product-fixture";

/// Set while a session is starting or running, so one tile cannot create two
/// children.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// The product window handle, so its destruction can release [`ACTIVE`].
static WINDOW: AtomicIsize = AtomicIsize::new(0);

/// A started session waiting for the UI thread to collect it.
static STARTED: OnceLock<Mutex<Option<RunningProductSession>>> = OnceLock::new();

/// Starts one product session on a worker, if none is already active.
///
/// Returns `false` when a session is already starting or running. `notify` is
/// called on the worker thread once the attempt finishes, successfully or not,
/// so the caller can post its own private window message.
pub(super) fn request_start(notify: impl FnOnce() + Send + 'static) -> bool {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return false;
    }
    let session_id = format!("product-lab-{}", std::process::id());
    let spawned = std::thread::Builder::new()
        .name("anodrel-product-tile".to_owned())
        .spawn(move || {
            match start_registered_product_session(FIXTURE_APPLICATION_ID, HOST_NAME, session_id) {
                Ok(session) => {
                    if let Ok(mut slot) = started_slot().lock() {
                        *slot = Some(session);
                    }
                    // A session that cannot be handed over is dropped here,
                    // which requests its own full shutdown.
                }
                Err(_) => {
                    // The tile reports nothing: a verified launch can still fail
                    // for reasons the surface must not describe.
                    ACTIVE.store(false, Ordering::SeqCst);
                }
            }
            notify();
        });

    if spawned.is_err() {
        ACTIVE.store(false, Ordering::SeqCst);
        return false;
    }
    true
}

/// Collects a started session on the UI thread, if the worker produced one.
pub(super) fn take_started() -> Option<RunningProductSession> {
    started_slot().lock().ok().and_then(|mut slot| slot.take())
}

/// Records the window that now owns a collected session.
pub(super) fn note_window(window: Hwnd) {
    WINDOW.store(window, Ordering::SeqCst);
}

/// Releases the single-session guard when the product window is destroyed.
///
/// The session itself is owned by that window's view, so removing the view is
/// what requests shutdown of the child, pipe worker, and exit watcher.
pub(super) fn note_destroyed(window: Hwnd) {
    if WINDOW.swap(0, Ordering::SeqCst) == window {
        ACTIVE.store(false, Ordering::SeqCst);
    } else {
        WINDOW.store(0, Ordering::SeqCst);
    }
}

/// Abandons a start attempt that produced no window.
pub(super) fn release() {
    ACTIVE.store(false, Ordering::SeqCst);
}

fn started_slot() -> &'static Mutex<Option<RunningProductSession>> {
    STARTED.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::{ACTIVE, FIXTURE_APPLICATION_ID, WINDOW, note_destroyed, release, take_started};

    /// These tests share process-global lifecycle state.
    static EXCLUSIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn the_tile_can_only_activate_the_one_fixture_identity() {
        assert_eq!(FIXTURE_APPLICATION_ID, "org.anodrel.product-fixture");
        assert!(anodrel_application::is_valid_application_id(
            FIXTURE_APPLICATION_ID
        ));
    }

    #[test]
    fn only_the_owning_window_releases_the_single_session_guard() {
        let _exclusive = EXCLUSIVE.lock().expect("product tile tests are serialized");
        ACTIVE.store(true, Ordering::SeqCst);
        WINDOW.store(-501, Ordering::SeqCst);

        note_destroyed(-502);
        assert!(
            ACTIVE.load(Ordering::SeqCst),
            "an unrelated window must not free a running product session"
        );

        WINDOW.store(-501, Ordering::SeqCst);
        note_destroyed(-501);
        assert!(!ACTIVE.load(Ordering::SeqCst));
    }

    #[test]
    fn nothing_is_collected_before_a_worker_produces_a_session() {
        let _exclusive = EXCLUSIVE.lock().expect("product tile tests are serialized");
        assert!(take_started().is_none());
        release();
        assert!(!ACTIVE.load(Ordering::SeqCst));
    }
}
