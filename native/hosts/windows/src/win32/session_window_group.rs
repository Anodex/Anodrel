//! Native lifetime ownership for one authenticated session's logical views.
//!
//! The portable [`UiWindowGroup`] owns documents, revisions, input queues, and
//! the worker-to-UI creation handoff. This module owns the opposite side of
//! that seam: the private mapping from those logical identities to Win32
//! handles, group-wide shutdown observation, and the optional verified-product
//! lifetime. It deliberately does not expose a handle, mapping, or close state
//! to application or transport code.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

use anodrel_core::SessionCloseSignal;
use anodrel_ui_session::{UiWindowGroup, UiWindowId, UiWindowOpenRequest};
use anodrel_window::{WindowTitleProposal, compose};
use anodrel_windows_product_session::RunningProductSession;

use super::Hwnd;

/// Host-owned native state for one authenticated session's window group.
///
/// Every view in the group holds a cheap clone. The optional product-session
/// owner therefore survives a secondary close and is released only after the
/// final registered view leaves the view registry. No clone reveals native
/// state: each operation receives a host-known member and runs only on the UI
/// thread that owns the windows.
#[derive(Clone, Debug)]
pub(super) struct SessionWindowGroup {
    portable: UiWindowGroup<WindowTitleProposal>,
    close_signal: SessionCloseSignal,
    display_name: Option<String>,
    state: Arc<Mutex<State>>,
    /// Retained solely for host lifetime. `RunningProductSession` performs its
    /// complete verified-child and worker shutdown when the final group clone
    /// drops; a secondary view can never own that lifetime by itself.
    _product_session: Option<Arc<RunningProductSession>>,
}

#[derive(Debug, Default)]
struct State {
    windows: BTreeMap<UiWindowId, Hwnd>,
    closing: bool,
}

/// One private association between a logical view and its native host group.
///
/// This is kept inside the registry's `UiSessionView`, never given to an
/// application. It is intentionally cloneable because the registry makes paint
/// snapshots; only [`Self::on_native_destroy`] mutates the real mapping.
#[derive(Clone, Debug)]
pub(super) struct SessionWindowMember {
    group: SessionWindowGroup,
    id: UiWindowId,
}

/// One portable creation handoff paired with the private native group that can
/// complete it. The public protocol never sees this type or a native handle.
#[derive(Debug)]
pub(super) struct SessionWindowOpenRequest {
    group: SessionWindowGroup,
    request: UiWindowOpenRequest<WindowTitleProposal>,
}

impl SessionWindowGroup {
    /// Builds one host group without a verified child for a host-controlled
    /// development diagnostic or a future non-product authenticated session.
    pub(super) fn new(
        portable: UiWindowGroup<WindowTitleProposal>,
        close_signal: SessionCloseSignal,
        display_name: Option<String>,
    ) -> Self {
        Self {
            portable,
            close_signal,
            display_name,
            state: Arc::new(Mutex::new(State::default())),
            _product_session: None,
        }
    }

    /// Creates the group lifetime owner for one verified product session.
    ///
    /// The resources are cloned before the session moves into the retained
    /// `Arc`, so they stay tied to exactly the machine-validated registered
    /// session that the product launcher created.
    pub(super) fn for_product_session(session: RunningProductSession) -> Self {
        let (portable, close_signal, display_name) = {
            let ui = session.ui();
            (
                ui.window_group(),
                ui.close_signal(),
                Some(ui.display_name().to_owned()),
            )
        };
        Self {
            portable,
            close_signal,
            display_name,
            state: Arc::new(Mutex::new(State::default())),
            _product_session: Some(Arc::new(session)),
        }
    }

    /// Returns the private group member carried by one host-created view.
    pub(super) fn member(&self, id: UiWindowId) -> SessionWindowMember {
        SessionWindowMember {
            group: self.clone(),
            id,
        }
    }

    /// Takes one pending portable open request for the owning UI thread.
    ///
    /// Once shutdown begins, a pending worker is answered immediately instead
    /// of being allowed to wait for a UI thread that is dismantling its group.
    pub(super) fn take_open_request(&self) -> Option<SessionWindowOpenRequest> {
        if self.observe_shutdown() {
            return None;
        }
        self.portable
            .take_open_request()
            .map(|request| SessionWindowOpenRequest {
                group: self.clone(),
                request,
            })
    }

    /// Returns whether all remaining native views must now close.
    ///
    /// The first timer to consume the coalescing core signal latches native
    /// shutdown for every sibling. Subsequent timers observe the latched value,
    /// so consuming the signal in one view cannot strand the others.
    pub(super) fn observe_shutdown(&self) -> bool {
        if self.close_signal.take() {
            self.begin_shutdown();
        }
        lock(&self.state).closing
    }

    /// Starts host-owned group shutdown and wakes an in-flight open worker.
    ///
    /// The Group Lab uses this only while tearing down its diagnostic owner.
    /// Product sessions normally arrive here through their shared close signal.
    pub(super) fn request_shutdown(&self) {
        self.close_signal.request();
        self.begin_shutdown();
    }

    fn begin_shutdown(&self) {
        let became_closing = {
            let mut state = lock(&self.state);
            if state.closing {
                false
            } else {
                state.closing = true;
                true
            }
        };
        if became_closing {
            // This rolls back an uncreated view and wakes its worker. A request
            // already being created is handled by its later `complete` result.
            let _ = self.portable.cancel_open_request();
        }
    }

    fn register(&self, id: &UiWindowId, window: Hwnd) -> bool {
        let mut state = lock(&self.state);
        if state.closing
            || state.windows.contains_key(id)
            || state
                .windows
                .values()
                .any(|registered| *registered == window)
        {
            return false;
        }
        state.windows.insert(id.clone(), window);
        true
    }

    fn unregister(&self, id: &UiWindowId, window: Hwnd) {
        let removed = {
            let mut state = lock(&self.state);
            if state.windows.get(id).copied() == Some(window) {
                state.windows.remove(id);
                true
            } else {
                false
            }
        };
        if !removed {
            return;
        }
        if id.is_primary() {
            // A primary is the anchor for all current primary-only bridges. A
            // user closing it therefore ends this whole native group.
            self.close_signal.request();
            self.begin_shutdown();
        } else {
            // The native window is already gone. Only now may the portable
            // identity and its independent document/input state be released.
            let _ = self.portable.close_secondary(id);
        }
    }

    #[cfg(test)]
    fn native_window_count(&self) -> usize {
        lock(&self.state).windows.len()
    }
}

impl SessionWindowMember {
    /// Registers the newly created native window before it is shown.
    pub(super) fn register_native_window(&self, window: Hwnd) -> bool {
        self.group.register(&self.id, window)
    }

    /// Removes exactly this mapping after Windows has destroyed the window.
    ///
    /// Ordinary paint clones must not call this: they carry the member only so
    /// drawing can keep a stable immutable view snapshot. The registry invokes
    /// it once for the real removed view after it has released its own lock.
    pub(super) fn on_native_destroy(&self, window: Hwnd) {
        self.group.unregister(&self.id, window);
    }

    /// Checks and latches one group-wide session close on the owning UI thread.
    pub(super) fn observe_shutdown(&self) -> bool {
        self.group.observe_shutdown()
    }

    /// Takes one pending secondary-view creation handoff for this UI thread.
    pub(super) fn take_open_request(&self) -> Option<SessionWindowOpenRequest> {
        self.group.take_open_request()
    }
}

impl SessionWindowOpenRequest {
    /// Returns the host-composed caption for the prospective native window.
    pub(super) fn caption(&self) -> String {
        compose(self.request.context(), self.group.display_name.as_deref())
    }

    /// Returns the new view's independent portable resources.
    pub(super) fn resources(&self) -> anodrel_ui_session::UiWindowResources {
        self.request.resources().clone()
    }

    /// Returns the member that the prospective view must carry before registry
    /// insertion. It has no mapping until `register_native_window` succeeds.
    pub(super) fn member(&self) -> SessionWindowMember {
        self.group.member(self.request.resources().id().clone())
    }

    /// Completes native creation after the view is registered and before shown.
    ///
    /// A false result means the worker timed out or group shutdown cancelled
    /// the request. The caller must destroy its just-created native window.
    pub(super) fn complete(&self) -> bool {
        self.group.portable.complete_open(self.request.id(), true)
    }

    /// Fails creation before any native view was shown.
    pub(super) fn fail(&self) -> bool {
        self.group.portable.fail_open(self.request.id())
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread};

    use anodrel_core::SessionCloseSignal;
    use anodrel_ui_session::{UiWindowGroup, UiWindowGroupError, UiWindowId};
    use anodrel_window::WindowTitleProposal;

    use super::SessionWindowGroup;

    const DOCUMENT: &str = r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Secondary","fontSize":16,"tone":"primary"}}"#;

    fn group() -> SessionWindowGroup {
        SessionWindowGroup::new(
            UiWindowGroup::new(),
            SessionCloseSignal::default(),
            Some("Verified App".to_owned()),
        )
    }

    fn title(value: &str) -> WindowTitleProposal {
        WindowTitleProposal::new(value).expect("fixed title is valid")
    }

    #[test]
    fn maps_only_one_exact_native_window_per_logical_member() {
        let group = group();
        let primary = group.member(UiWindowId::primary());
        assert!(primary.register_native_window(-301));
        assert!(!primary.register_native_window(-301));
        assert_eq!(group.native_window_count(), 1);

        // A spurious destruction message for a different handle cannot release
        // the real mapping or make the group think its primary left.
        primary.on_native_destroy(-302);
        assert_eq!(group.native_window_count(), 1);
        assert!(!primary.observe_shutdown());

        primary.on_native_destroy(-301);
        assert_eq!(group.native_window_count(), 0);
        assert!(primary.observe_shutdown());
    }

    #[test]
    fn a_secondary_native_close_releases_only_its_portable_view() {
        let group = group();
        let portable = group.portable.clone();
        let worker = portable.clone();
        let (sent, received) = mpsc::channel();
        let waiting =
            thread::spawn(move || sent.send(worker.open_secondary(title("Notes"), DOCUMENT)));

        let request = loop {
            if let Some(request) = group.take_open_request() {
                break request;
            }
            thread::yield_now();
        };
        assert_eq!(request.caption(), "Notes — Verified App");
        let member = request.member();
        assert!(member.register_native_window(-311));
        assert!(request.complete());
        let id = received
            .recv()
            .expect("worker returns a response")
            .expect("secondary was committed");
        waiting
            .join()
            .expect("worker does not panic")
            .expect("worker submits its response");
        assert!(portable.contains(&id));

        member.on_native_destroy(-311);
        assert!(!portable.contains(&id));
        assert!(!group.member(UiWindowId::primary()).observe_shutdown());
    }

    #[test]
    fn primary_close_cancels_a_taken_open_handoff_without_waiting() {
        let group = group();
        let primary = group.member(UiWindowId::primary());
        assert!(primary.register_native_window(-321));
        let worker = group.portable.clone();
        let (sent, received) = mpsc::channel();
        let waiting =
            thread::spawn(move || sent.send(worker.open_secondary(title("Draft"), DOCUMENT)));

        let request = loop {
            if let Some(request) = group.take_open_request() {
                break request;
            }
            thread::yield_now();
        };
        primary.on_native_destroy(-321);
        assert!(primary.observe_shutdown());
        assert_eq!(
            received.recv().expect("worker receives shutdown"),
            Err(UiWindowGroupError::Unavailable)
        );
        waiting
            .join()
            .expect("worker does not panic")
            .expect("worker submits its response");
        assert!(
            !request.complete(),
            "a late UI thread must destroy instead of publishing a rolled-back view"
        );
    }
}
