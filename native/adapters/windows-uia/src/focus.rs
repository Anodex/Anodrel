//! A bounded route from UI Automation focus requests to one host UI thread.
//!
//! UI Automation can call `SetFocus` from a thread that does not own the
//! custom-drawn view. This module gives that caller one short-lived request
//! slot, while the host remains the only code that can validate a current
//! layout and write its live focus state. It has no protocol, application, or
//! native-input authority. See Decision 0073.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use anodrel_ui::ElementId;
use anodrel_ui_session::UiDocumentRevision;

/// Maximum time an automation caller waits for the owning UI thread.
///
/// `SetFocus` should be immediate from a person's perspective. A longer wait
/// would only turn a stalled UI thread into a stalled screen-reader call.
pub const UI_AUTOMATION_FOCUS_RESPONSE_TIMEOUT: Duration = Duration::from_millis(250);

/// A host-owned one-request focus route for one native view.
#[derive(Clone, Debug, Default)]
pub struct UiAutomationFocusMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: UiAutomationFocusRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<bool>>,
    ready: Condvar,
}

/// One target waiting to be revalidated by its owning UI thread.
#[derive(Clone, Debug)]
pub struct UiAutomationFocusRequest {
    id: u64,
    revision: Option<UiDocumentRevision>,
    target: ElementId,
}

impl UiAutomationFocusRequest {
    /// Returns the identity used only to complete this exact route entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the document revision published by the calling provider.
    ///
    /// `None` identifies a fixed host diagnostic instead of an authenticated
    /// application document.
    #[must_use]
    pub const fn revision(&self) -> Option<UiDocumentRevision> {
        self.revision
    }

    /// Returns the already-published semantic target the host must revalidate.
    #[must_use]
    pub const fn target(&self) -> &ElementId {
        &self.target
    }
}

/// A revision-bound focus route before it is attached to one native window.
///
/// The host creates a fresh route for every UI Automation provider snapshot.
/// Binding a route is private host composition, not a public window handle API.
#[derive(Clone, Debug)]
pub struct UiAutomationFocusRoute {
    mailbox: UiAutomationFocusMailbox,
    revision: Option<UiDocumentRevision>,
}

impl UiAutomationFocusRoute {
    /// Binds this snapshot's focus route to a host-owned private message.
    #[must_use]
    pub fn for_window(&self, window: isize, wake_message: u32) -> UiAutomationFocusSink {
        UiAutomationFocusSink {
            mailbox: self.mailbox.clone(),
            revision: self.revision,
            wake: Wake::Window {
                window,
                message: wake_message,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn with_notifier(
        &self,
        notifier: impl Fn() -> bool + Send + Sync + 'static,
    ) -> UiAutomationFocusSink {
        UiAutomationFocusSink {
            mailbox: self.mailbox.clone(),
            revision: self.revision,
            wake: Wake::Notifier(Arc::new(notifier)),
        }
    }
}

/// The UI Automation provider's bounded focus route.
///
/// It carries a semantic ID and one snapshot revision, never a mutable view,
/// registry entry, native input message, or application callback.
#[derive(Clone)]
pub struct UiAutomationFocusSink {
    mailbox: UiAutomationFocusMailbox,
    revision: Option<UiDocumentRevision>,
    wake: Wake,
}

impl std::fmt::Debug for UiAutomationFocusSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UiAutomationFocusSink")
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
enum Wake {
    /// A private, payload-free message dispatched by the window owner.
    Window { window: isize, message: u32 },
    #[cfg(test)]
    Notifier(Arc<dyn Fn() -> bool + Send + Sync>),
}

impl UiAutomationFocusSink {
    /// Offers one published target to the owner and waits for its answer.
    ///
    /// A `false` result deliberately combines a stale provider, unavailable
    /// window, full route, timeout, and host refusal. UI Automation receives
    /// no data about the view, session, or reason for the refusal.
    pub(crate) fn focus(&self, target: ElementId) -> bool {
        self.mailbox.request_within(
            self.revision,
            target,
            UI_AUTOMATION_FOCUS_RESPONSE_TIMEOUT,
            || self.wake(),
        )
    }

    fn wake(&self) -> bool {
        match &self.wake {
            Wake::Window { window, message } => {
                let Some(owner) = owner_thread(*window) else {
                    // A provider can outlive a destroyed HWND. Do not post the
                    // private message if Windows has invalidated that handle or
                    // reused it outside this host process.
                    return false;
                };
                // UI Automation can call a provider directly from the window's
                // owner thread. Posting there would make that thread wait for a
                // message loop it is currently running, so dispatch the same
                // payload-free message synchronously in that one case.
                if owner == current_thread() {
                    // SAFETY: the host created `window` and handles this fixed
                    // private message without reading either payload word.
                    unsafe {
                        SendMessageW(*window, *message, 0, 0);
                    }
                    true
                } else {
                    // SAFETY: the fixed message carries no pointer, element
                    // identifier, or application data. The host looks up only
                    // its own active route request when it receives it.
                    unsafe { PostMessageW(*window, *message, 0, 0) != 0 }
                }
            }
            #[cfg(test)]
            Wake::Notifier(notifier) => notifier(),
        }
    }
}

impl UiAutomationFocusMailbox {
    /// Creates an empty route for one host-owned native view.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates one provider-snapshot route over this view's request slot.
    #[must_use]
    pub fn route(&self, revision: Option<UiDocumentRevision>) -> UiAutomationFocusRoute {
        UiAutomationFocusRoute {
            mailbox: self.clone(),
            revision,
        }
    }

    /// Takes the current request exactly once on the owning UI thread.
    #[must_use]
    pub fn take(&self) -> Option<UiAutomationFocusRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Completes one taken request after the host accepted or rejected it.
    ///
    /// A late, unknown, or untaken completion returns `false` and cannot answer
    /// another caller's request.
    pub fn complete(&self, request_id: u64, accepted: bool) -> bool {
        self.complete_with(request_id, || accepted).is_some()
    }

    /// Applies one host-side focus decision only while its caller still waits.
    ///
    /// The decision runs while this route still owns the exact request slot.
    /// That closes the timeout race: a UI thread that arrives after the caller
    /// has given up cannot change focus and then discover its completion was
    /// too late. `None` means this was an unknown, untaken, or expired request,
    /// and `decision` was not called.
    pub fn complete_with(&self, request_id: u64, decision: impl FnOnce() -> bool) -> Option<bool> {
        let response = {
            let state = &mut *lock(&self.state);
            let active = state.active.as_ref()?;
            if active.request.id != request_id || !active.taken {
                return None;
            }
            // Keep the request live while the owning UI thread validates and
            // applies it. The work is bounded pure view state, never an OS
            // call or callback, so this small critical section cannot stall a
            // route behind external work.
            let accepted = decision();
            let response = Arc::clone(&active.response);
            state.active = None;
            (response, accepted)
        };
        let (response, accepted) = response;
        let value = &mut *lock(&response.value);
        *value = Some(accepted);
        response.ready.notify_one();
        Some(accepted)
    }

    fn request_within(
        &self,
        revision: Option<UiDocumentRevision>,
        target: ElementId,
        timeout: Duration,
        wake: impl FnOnce() -> bool,
    ) -> bool {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return false;
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: UiAutomationFocusRequest {
                    id: request_id,
                    revision,
                    target,
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        if !wake() {
            self.clear(request_id);
            return false;
        }

        let response_value = wait_for_response(&response, timeout);
        if response_value.is_none() {
            // A late UI-thread completion must not reach a later request, and
            // an unavailable view must not leave UI Automation permanently busy.
            self.clear(request_id);
        }
        response_value.unwrap_or(false)
    }

    fn clear(&self, request_id: u64) {
        let state = &mut *lock(&self.state);
        if state
            .active
            .as_ref()
            .is_some_and(|active| active.request.id == request_id)
        {
            state.active = None;
        }
    }
}

fn wait_for_response(response: &ResponseSlot, timeout: Duration) -> Option<bool> {
    let value = lock(&response.value);
    let (mut value, timed_out) = response
        .ready
        .wait_timeout_while(value, timeout, |value| value.is_none())
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if timed_out.timed_out() && value.is_none() {
        None
    } else {
        value.take()
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn GetCurrentThreadId() -> u32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowThreadProcessId(window: isize, process_id: *mut u32) -> u32;
    fn PostMessageW(window: isize, message: u32, wparam: usize, lparam: isize) -> i32;
    fn SendMessageW(window: isize, message: u32, wparam: usize, lparam: isize) -> isize;
}

fn current_thread() -> u32 {
    // SAFETY: this has no pointers or preconditions.
    unsafe { GetCurrentThreadId() }
}

fn owner_thread(window: isize) -> Option<u32> {
    let mut process_id = 0;
    // SAFETY: `process_id` is writable and this query has no other
    // preconditions. A destroyed or foreign HWND must not receive the host's
    // private wakeup, even if Windows later reuses its numeric value.
    let thread_id = unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    (thread_id != 0 && process_id == current_process()).then_some(thread_id)
}

fn current_process() -> u32 {
    // SAFETY: this has no pointers or preconditions.
    unsafe { GetCurrentProcessId() }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use anodrel_ui::ElementId;
    use anodrel_ui_session::{UiDocumentRevision, UiDocumentSession};

    use super::{UiAutomationFocusMailbox, owner_thread};

    fn id() -> ElementId {
        ElementId::new("submit").expect("fixed ID is valid")
    }

    fn revision() -> UiDocumentRevision {
        UiDocumentSession::new()
            .replace_document(
                r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Focus","fontSize":16,"tone":"primary"}}"#,
            )
            .expect("the fixed document is valid")
    }

    fn take_pending(mailbox: &UiAutomationFocusMailbox) -> super::UiAutomationFocusRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_revision_bound_target_to_its_owner() {
        let mailbox = UiAutomationFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.request_within(Some(revision()), id(), Duration::from_secs(1), || true)
        });

        let request = take_pending(&mailbox);
        assert_eq!(request.revision(), Some(revision()));
        assert_eq!(request.target(), &id());
        assert!(mailbox.complete(request.id(), true));
        assert!(waiting.join().expect("caller did not panic"));
    }

    #[test]
    fn busy_or_unknown_routes_cannot_change_an_active_request() {
        let mailbox = UiAutomationFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.request_within(Some(revision()), id(), Duration::from_secs(1), || true)
        });
        let request = take_pending(&mailbox);

        assert!(
            !mailbox.request_within(Some(revision()), id(), Duration::ZERO, || true),
            "a second automation caller entered an occupied route"
        );
        assert!(!mailbox.complete(request.id().saturating_add(1), true));
        assert!(mailbox.complete(request.id(), false));
        assert!(!waiting.join().expect("caller did not panic"));
    }

    #[test]
    fn timeout_or_failed_wakeup_releases_the_exact_route_slot() {
        let mailbox = UiAutomationFocusMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.request_within(Some(revision()), id(), Duration::from_millis(20), || true)
        });
        let abandoned = take_pending(&mailbox);
        assert!(!waiting.join().expect("caller did not panic"));
        assert!(
            !mailbox.complete(abandoned.id(), true),
            "a late completion answered a timed-out caller"
        );
        let mut applied = false;
        assert_eq!(
            mailbox.complete_with(abandoned.id(), || {
                applied = true;
                true
            }),
            None,
            "an expired request reached host focus state"
        );
        assert!(!applied, "an expired route ran its focus decision");

        let worker = mailbox.clone();
        let next = thread::spawn(move || {
            worker.request_within(None, id(), Duration::from_secs(1), || false)
        });
        assert!(!next.join().expect("caller did not panic"));

        let worker = mailbox.clone();
        let next = thread::spawn(move || {
            worker.request_within(None, id(), Duration::from_secs(1), || true)
        });
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id(), true));
        assert!(next.join().expect("caller did not panic"));
    }

    #[test]
    fn a_test_notifier_can_complete_a_route_without_a_window() {
        let mailbox = UiAutomationFocusMailbox::new();
        let route = mailbox.route(Some(revision()));
        let completing = mailbox.clone();
        let sink = route.with_notifier(move || {
            let request = completing.take().expect("route request is pending");
            completing.complete(request.id(), true)
        });

        assert!(sink.focus(id()));
    }

    #[test]
    fn an_invalid_window_is_never_a_focus_route_owner() {
        assert_eq!(owner_thread(0), None);
    }
}
