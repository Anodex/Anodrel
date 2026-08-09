//! One bounded, host-UI-thread notification bridge.
//!
//! A notification reaches the operating system through Shell32, and the rule
//! that keeps a pipe worker away from User32 keeps it away from Shell32 too. The
//! authenticated worker therefore hands one validated notification to this
//! mailbox, and the owning native UI thread performs the call.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

use crate::{Notification, NotificationService, NotificationServiceError};

/// Maximum time a protocol worker may wait for its host UI thread to respond.
///
/// This is far shorter than the file-dialog bridge's timeout, and deliberately
/// so: a dialog waits on a person deciding, while a notification waits only on
/// a shell call that should complete in milliseconds. A worker blocked longer
/// than this is not being patient, it is stuck.
pub const NOTIFICATION_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// A bounded pending request taken exactly once by the host UI thread.
#[derive(Clone, Debug)]
pub struct NotificationRequest {
    id: u64,
    notification: Notification,
}

impl NotificationRequest {
    /// Returns the request identity used only to complete this mailbox entry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the validated notification the host UI thread must show.
    #[must_use]
    pub const fn notification(&self) -> &Notification {
        &self.notification
    }
}

/// A one-request bridge from a protocol worker to one host UI thread.
///
/// At most one notification may be pending or displayed at a time. A second
/// request while one is pending is refused as [`NotificationServiceError::Busy`],
/// and a request the UI thread never completes fails safely once the timeout
/// above elapses.
#[derive(Clone, Debug, Default)]
pub struct NotificationMailbox {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    active: Option<ActiveRequest>,
}

#[derive(Debug)]
struct ActiveRequest {
    request: NotificationRequest,
    taken: bool,
    response: Arc<ResponseSlot>,
}

#[derive(Debug, Default)]
struct ResponseSlot {
    value: Mutex<Option<Result<(), NotificationServiceError>>>,
    ready: Condvar,
}

impl NotificationMailbox {
    /// Creates an empty UI-thread notification mailbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the current request once so the owning native UI thread can show it.
    #[must_use]
    pub fn take(&self) -> Option<NotificationRequest> {
        let state = &mut *lock(&self.state);
        let active = state.active.as_mut()?;
        if active.taken {
            None
        } else {
            active.taken = true;
            Some(active.request.clone())
        }
    }

    /// Reports that the host UI thread handed the notification to the system.
    ///
    /// Returns `false` when the request expired or was not the active request.
    /// Acceptance means handed over, never that the user saw anything.
    pub fn complete(&self, request_id: u64) -> bool {
        self.respond(request_id, Ok(()))
    }

    /// Completes the matching request with a safe unavailable result.
    ///
    /// Returns `false` when the request expired or was not the active request.
    pub fn fail(&self, request_id: u64) -> bool {
        self.respond(request_id, Err(NotificationServiceError::Unavailable))
    }

    fn show_within(
        &self,
        notification: &Notification,
        timeout: Duration,
    ) -> Result<(), NotificationServiceError> {
        let response = Arc::new(ResponseSlot::default());
        let request_id = {
            let state = &mut *lock(&self.state);
            if state.active.is_some() {
                return Err(NotificationServiceError::Busy);
            }
            state.next_id = state.next_id.checked_add(1).unwrap_or(1);
            let request_id = state.next_id;
            state.active = Some(ActiveRequest {
                request: NotificationRequest {
                    id: request_id,
                    notification: notification.clone(),
                },
                taken: false,
                response: Arc::clone(&response),
            });
            request_id
        };

        let result = wait_for_response(&response, timeout);
        if result.is_none() {
            // The UI thread never answered. Clear the slot so this session is
            // not left permanently busy by one stuck request.
            let state = &mut *lock(&self.state);
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.request.id == request_id)
            {
                state.active = None;
            }
        }
        result.unwrap_or(Err(NotificationServiceError::Unavailable))
    }

    fn respond(&self, request_id: u64, result: Result<(), NotificationServiceError>) -> bool {
        let response = {
            let state = &mut *lock(&self.state);
            let Some(active) = state.active.as_ref() else {
                return false;
            };
            if active.request.id != request_id || !active.taken {
                return false;
            }
            let response = Arc::clone(&active.response);
            state.active = None;
            response
        };
        let value = &mut *lock(&response.value);
        *value = Some(result);
        response.ready.notify_one();
        true
    }
}

impl NotificationService for NotificationMailbox {
    fn show(&self, notification: &Notification) -> Result<(), NotificationServiceError> {
        self.show_within(notification, NOTIFICATION_RESPONSE_TIMEOUT)
    }
}

fn wait_for_response(
    response: &ResponseSlot,
    timeout: Duration,
) -> Option<Result<(), NotificationServiceError>> {
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

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::{NotificationMailbox, NotificationService, NotificationServiceError};
    use crate::{Notification, NotificationBody, NotificationTitle};

    fn notification() -> Notification {
        Notification::new(
            NotificationTitle::new("Build finished").expect("the test title is valid"),
            NotificationBody::new("Two targets").expect("the test body is valid"),
        )
    }

    /// Spins until the UI-thread side can take the pending request.
    fn take_pending(mailbox: &NotificationMailbox) -> super::NotificationRequest {
        loop {
            if let Some(request) = mailbox.take() {
                return request;
            }
            thread::yield_now();
        }
    }

    #[test]
    fn transfers_one_notification_to_the_ui_thread_and_reports_acceptance() {
        let mailbox = NotificationMailbox::new();
        let worker = mailbox.clone();
        let (sent, received) = mpsc::channel();
        let worker_thread = thread::spawn(move || sent.send(worker.show(&notification())));

        let request = take_pending(&mailbox);
        assert_eq!(request.notification(), &notification());
        // A request is handed to the UI thread exactly once.
        assert!(mailbox.take().is_none());
        assert!(mailbox.complete(request.id()));

        assert_eq!(received.recv().expect("the worker responded"), Ok(()));
        worker_thread
            .join()
            .expect("the worker did not panic")
            .expect("the receiver remained available");
    }

    #[test]
    fn refuses_a_second_notification_as_busy_rather_than_unavailable() {
        // Busy and Unavailable are different answers: one means try again, the
        // other means this host cannot show notifications at all.
        let mailbox = NotificationMailbox::new();
        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.show(&notification()));

        let request = take_pending(&mailbox);
        assert_eq!(
            mailbox.show(&notification()),
            Err(NotificationServiceError::Busy)
        );
        assert!(mailbox.complete(request.id()));
        assert_eq!(waiting.join().expect("the worker did not panic"), Ok(()));
    }

    #[test]
    fn ignores_a_completion_for_an_untaken_or_unknown_request() {
        let mailbox = NotificationMailbox::new();
        assert!(!mailbox.complete(1));
        assert!(!mailbox.fail(1));

        let worker = mailbox.clone();
        let waiting = thread::spawn(move || worker.show(&notification()));
        // Completing before the UI thread has taken it would let a response
        // race ahead of the call it is supposed to describe.
        while mailbox.take().is_none() {
            thread::yield_now();
        }
        assert!(!mailbox.complete(9_999));
        assert!(mailbox.fail(1));
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(NotificationServiceError::Unavailable)
        );
    }

    #[test]
    fn a_request_the_ui_thread_never_answers_frees_the_session_again() {
        let mailbox = NotificationMailbox::new();
        let worker = mailbox.clone();
        let waiting =
            thread::spawn(move || worker.show_within(&notification(), Duration::from_millis(20)));

        let _abandoned = take_pending(&mailbox);
        assert_eq!(
            waiting.join().expect("the worker did not panic"),
            Err(NotificationServiceError::Unavailable)
        );

        // The session must not be left permanently busy by one stuck request.
        let worker = mailbox.clone();
        let next = thread::spawn(move || worker.show(&notification()));
        let request = take_pending(&mailbox);
        assert!(mailbox.complete(request.id()));
        assert_eq!(next.join().expect("the worker did not panic"), Ok(()));
    }
}
