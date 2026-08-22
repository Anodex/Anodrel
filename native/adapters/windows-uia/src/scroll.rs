//! A bounded route from UI Automation scrolling to one host UI thread.
//!
//! UI Automation may call `IScrollProvider` from a thread that does not own the
//! custom-drawn view. The provider has one short-lived request slot; only the
//! owning thread validates a current layout and changes host-retained scroll
//! state. It has no protocol, application, or native-input authority. See
//! Decision 0097.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use anodrel_ui::ElementId;
use anodrel_ui_session::UiDocumentRevision;

/// Maximum time an automation caller waits for the owning UI thread.
pub const UI_AUTOMATION_SCROLL_RESPONSE_TIMEOUT: Duration = Duration::from_millis(250);

/// One closed vertical movement command a provider may offer to its host.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAutomationScrollCommand {
    /// Moves by the host's standard local line amount.
    Line {
        /// Whether the movement goes toward the document's end.
        forward: bool,
    },
    /// Moves by the current viewport size.
    Page {
        /// Whether the movement goes toward the document's end.
        forward: bool,
    },
    /// Moves to a vertical percentage in the closed range 0 through 100.
    Percent {
        /// The requested percentage.
        percent: f64,
    },
    /// Reveals one permitted semantic descendant of the selected viewport.
    ///
    /// The owner revalidates both this item and the enclosing viewport against
    /// its current document and layout before it changes host-retained state.
    ScrollIntoView {
        /// The semantic item that should become visible.
        item: ElementId,
    },
}

/// The finite immutable vertical state a provider publishes.
#[derive(Clone, Debug, PartialEq)]
pub struct UiAutomationScrollSnapshot {
    target: ElementId,
    vertical_scroll_percent: f64,
    vertical_view_size: f64,
}

impl UiAutomationScrollSnapshot {
    /// Builds a snapshot for one genuinely overflowing vertical viewport.
    #[must_use]
    pub fn new(
        target: ElementId,
        viewport_height: f32,
        content_height: f32,
        offset_y: f32,
    ) -> Option<Self> {
        let viewport = f64::from(viewport_height);
        let content = f64::from(content_height);
        let offset = f64::from(offset_y);
        if !viewport.is_finite()
            || !content.is_finite()
            || !offset.is_finite()
            || viewport <= 0.0
            || content <= viewport
        {
            return None;
        }
        let maximum = content - viewport;
        Some(Self {
            target,
            vertical_scroll_percent: (offset.clamp(0.0, maximum) / maximum) * 100.0,
            vertical_view_size: ((viewport / content) * 100.0).clamp(0.0, 100.0),
        })
    }

    /// Returns the selected semantic viewport identity.
    #[must_use]
    pub const fn target(&self) -> &ElementId {
        &self.target
    }

    /// Returns the copied vertical scroll percentage.
    #[must_use]
    pub const fn vertical_scroll_percent(&self) -> f64 {
        self.vertical_scroll_percent
    }

    /// Returns the copied vertical view-size percentage.
    #[must_use]
    pub const fn vertical_view_size(&self) -> f64 {
        self.vertical_view_size
    }
}

/// A host-owned one-request scroll route for one native view.
#[derive(Clone, Debug, Default)]
pub struct UiAutomationScrollMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: UiAutomationScrollRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<bool>>,
    ready: Condvar,
}

/// One closed command waiting to be revalidated by its owning UI thread.
#[derive(Clone, Debug)]
pub struct UiAutomationScrollRequest {
    id: u64,
    revision: Option<UiDocumentRevision>,
    target: ElementId,
    command: UiAutomationScrollCommand,
}

impl UiAutomationScrollRequest {
    /// Returns the identity used only to complete this exact route entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the provider revision, if this is an authenticated session.
    #[must_use]
    pub const fn revision(&self) -> Option<UiDocumentRevision> {
        self.revision
    }

    /// Returns the selected scroll viewport identity for revalidation.
    #[must_use]
    pub const fn target(&self) -> &ElementId {
        &self.target
    }

    /// Returns the closed host command.
    #[must_use]
    pub fn command(&self) -> UiAutomationScrollCommand {
        self.command.clone()
    }
}

/// A revision-bound route before the host attaches it to a private window.
#[derive(Clone, Debug)]
pub struct UiAutomationScrollRoute {
    mailbox: UiAutomationScrollMailbox,
    revision: Option<UiDocumentRevision>,
}

impl UiAutomationScrollRoute {
    /// Binds this route to one payload-free host-private wake message.
    #[must_use]
    pub fn for_window(&self, window: isize, wake_message: u32) -> UiAutomationScrollSink {
        UiAutomationScrollSink {
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
    ) -> UiAutomationScrollSink {
        UiAutomationScrollSink {
            mailbox: self.mailbox.clone(),
            revision: self.revision,
            wake: Wake::Notifier(Arc::new(notifier)),
        }
    }
}

/// The provider's bounded route back to the host's current scroll state.
#[derive(Clone)]
pub struct UiAutomationScrollSink {
    mailbox: UiAutomationScrollMailbox,
    revision: Option<UiDocumentRevision>,
    wake: Wake,
}

impl std::fmt::Debug for UiAutomationScrollSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UiAutomationScrollSink")
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
enum Wake {
    Window {
        window: isize,
        message: u32,
    },
    #[cfg(test)]
    Notifier(Arc<dyn Fn() -> bool + Send + Sync>),
}

impl UiAutomationScrollSink {
    /// Offers one closed command to the owner and waits for a generic answer.
    pub(crate) fn scroll(&self, target: ElementId, command: UiAutomationScrollCommand) -> bool {
        self.mailbox.request_within(
            self.revision,
            target,
            command,
            UI_AUTOMATION_SCROLL_RESPONSE_TIMEOUT,
            || self.wake(),
        )
    }

    fn wake(&self) -> bool {
        match &self.wake {
            Wake::Window { window, message } => {
                let Some(owner) = owner_thread(*window) else {
                    return false;
                };
                if owner == current_thread() {
                    // SAFETY: the host created this window and handles the
                    // fixed private message without reading either payload.
                    unsafe {
                        SendMessageW(*window, *message, 0, 0);
                    }
                    true
                } else {
                    // SAFETY: the fixed message carries no element identity,
                    // command, pointer, or application data.
                    unsafe { PostMessageW(*window, *message, 0, 0) != 0 }
                }
            }
            #[cfg(test)]
            Wake::Notifier(notifier) => notifier(),
        }
    }
}

impl UiAutomationScrollMailbox {
    /// Creates an empty route for one host-owned native view.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates one provider-snapshot route over this view's request slot.
    #[must_use]
    pub fn route(&self, revision: Option<UiDocumentRevision>) -> UiAutomationScrollRoute {
        UiAutomationScrollRoute {
            mailbox: self.clone(),
            revision,
        }
    }

    /// Takes the active request exactly once on the owning UI thread.
    #[must_use]
    pub fn take(&self) -> Option<UiAutomationScrollRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Applies a host decision only while this exact caller still waits.
    pub fn complete_with(&self, request_id: u64, decision: impl FnOnce() -> bool) -> Option<bool> {
        let (response, accepted) = {
            let state = &mut *lock(&self.state);
            let active = state.active.as_ref()?;
            if active.request.id != request_id || !active.taken {
                return None;
            }
            let accepted = decision();
            let response = Arc::clone(&active.response);
            state.active = None;
            (response, accepted)
        };
        let value = &mut *lock(&response.value);
        *value = Some(accepted);
        response.ready.notify_one();
        Some(accepted)
    }

    fn request_within(
        &self,
        revision: Option<UiDocumentRevision>,
        target: ElementId,
        command: UiAutomationScrollCommand,
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
                request: UiAutomationScrollRequest {
                    id: request_id,
                    revision,
                    target,
                    command,
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
    // SAFETY: process_id is writable and this query has no other preconditions.
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

    use super::{
        UiAutomationScrollCommand, UiAutomationScrollMailbox, UiAutomationScrollSnapshot,
        owner_thread,
    };

    fn id() -> ElementId {
        ElementId::new("viewport").expect("fixed ID is valid")
    }

    fn revision() -> UiDocumentRevision {
        UiDocumentSession::new()
            .replace_document(
                r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Scroll","fontSize":16,"tone":"primary"}}"#,
            )
            .expect("the fixed document is valid")
    }

    fn take_pending(mailbox: &UiAutomationScrollMailbox) -> super::UiAutomationScrollRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn snapshot_clamps_a_finite_vertical_position_and_rejects_non_overflow() {
        let snapshot = UiAutomationScrollSnapshot::new(id(), 25.0, 100.0, 500.0)
            .expect("a real overflow has a snapshot");
        assert_eq!(snapshot.target(), &id());
        assert_eq!(snapshot.vertical_scroll_percent(), 100.0);
        assert_eq!(snapshot.vertical_view_size(), 25.0);
        assert!(UiAutomationScrollSnapshot::new(id(), 100.0, 100.0, 0.0).is_none());
        assert!(UiAutomationScrollSnapshot::new(id(), f32::NAN, 100.0, 0.0).is_none());
    }

    #[test]
    fn transfers_one_revision_bound_command_to_its_owner() {
        let mailbox = UiAutomationScrollMailbox::new();
        let worker = mailbox.clone();
        let command = UiAutomationScrollCommand::Page { forward: true };
        let command_for_worker = command.clone();
        let waiting = thread::spawn(move || {
            worker.request_within(
                Some(revision()),
                id(),
                command_for_worker,
                Duration::from_secs(1),
                || true,
            )
        });
        let request = take_pending(&mailbox);
        assert_eq!(request.revision(), Some(revision()));
        assert_eq!(request.target(), &id());
        assert_eq!(request.command(), command);
        assert_eq!(mailbox.complete_with(request.id(), || true), Some(true));
        assert!(waiting.join().expect("caller did not panic"));
    }

    #[test]
    fn busy_and_timed_out_routes_cannot_apply_a_late_command() {
        let mailbox = UiAutomationScrollMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || {
            worker.request_within(
                Some(revision()),
                id(),
                UiAutomationScrollCommand::Line { forward: true },
                Duration::from_millis(20),
                || true,
            )
        });
        let request = take_pending(&mailbox);
        assert!(!waiting.join().expect("caller did not panic"));
        let mut applied = false;
        assert_eq!(
            mailbox.complete_with(request.id(), || {
                applied = true;
                true
            }),
            None
        );
        assert!(!applied);
        assert!(!mailbox.request_within(
            None,
            id(),
            UiAutomationScrollCommand::Percent { percent: 20.0 },
            Duration::ZERO,
            || false,
        ));
    }

    #[test]
    fn a_test_notifier_can_complete_a_scroll_without_a_window() {
        let mailbox = UiAutomationScrollMailbox::new();
        let route = mailbox.route(None);
        let completing = mailbox.clone();
        let sink = route.with_notifier(move || {
            let request = completing.take().expect("scroll request is pending");
            completing.complete_with(request.id(), || true).is_some()
        });
        assert!(sink.scroll(id(), UiAutomationScrollCommand::Line { forward: false }));
    }

    #[test]
    fn an_invalid_window_is_never_a_scroll_route_owner() {
        assert_eq!(owner_thread(0), None);
    }
}
