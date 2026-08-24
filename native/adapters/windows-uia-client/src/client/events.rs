//! One private-focus-event subscription for host acceptance diagnostics.
//!
//! The production host never subscribes to its own accessibility events. This
//! direct client owns a short-lived COM callback only while a host diagnostic
//! verifies that Windows received one outbound focus notification.

use std::{
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use crate::{com::succeeded, raw};

use super::{UiAutomationClient, UiAutomationError, text_property_from_raw};

/// One short-lived focus-event listener owned by a host acceptance diagnostic.
///
/// It has no application protocol or SDK connection. The caller arms exactly
/// one observation, triggers its own fixed host action, then waits for one
/// compiled AutomationId before this guard unregisters and releases the COM
/// callback.
pub struct UiAutomationFocusSubscription<'automation> {
    client: &'automation UiAutomationClient,
    handler: NonNull<FocusEventHandler>,
    state: Arc<FocusEventState>,
}

impl<'automation> UiAutomationFocusSubscription<'automation> {
    /// Starts one fresh private focus-event observation.
    ///
    /// The caller supplies no listener, callback, or event target from an
    /// application. It only controls the fixed diagnostic sequence that it
    /// itself immediately follows with a host-selected focus request.
    pub fn arm(&self) {
        self.state.arm();
    }

    /// Waits for the one armed focus event to name the expected fixed target.
    ///
    /// An absent, malformed, or differently named sender is a failed fixed
    /// diagnostic contract. The observed identifier is never returned to an
    /// application, protocol caller, or SDK consumer.
    pub fn wait_for_automation_id(
        &self,
        expected: &str,
        timeout: Duration,
    ) -> Result<(), UiAutomationError> {
        let observed = self.state.wait_for_automation_id(timeout)?;
        if observed == expected {
            Ok(())
        } else {
            Err(UiAutomationError::UnexpectedTree)
        }
    }
}

impl Drop for UiAutomationFocusSubscription<'_> {
    fn drop(&mut self) {
        self.state.disarm();
        // SAFETY: `client` and `handler` remain live for this guard's whole
        // lifetime. Removal ends Windows' registration before this guard
        // releases its own COM reference. A failed best-effort removal cannot
        // be reported from Drop and Windows still owns any reference it kept.
        unsafe {
            let vtable = (*self.client.automation.as_ptr()).vtable;
            let _ = ((*vtable).remove_focus_changed_event_handler)(
                self.client.automation.as_ptr(),
                self.handler.as_ptr().cast(),
            );
            release_handler(self.handler.as_ptr());
        }
    }
}

impl UiAutomationClient {
    /// Registers one private focus-event callback for a host acceptance check.
    ///
    /// The callback copies only one sender AutomationId into a bounded local
    /// slot. It exposes no application-selectable scope, element, cache,
    /// listener, provider, or callback result.
    pub fn subscribe_to_focus_changes(
        &self,
    ) -> Result<UiAutomationFocusSubscription<'_>, UiAutomationError> {
        let state = Arc::new(FocusEventState::default());
        let handler = FocusEventHandler::new(Arc::clone(&state));
        // SAFETY: `automation` and the handler's first field are live COM
        // interfaces. A null cache request asks Windows for the sender's
        // current properties, and only this subscription owns the handler.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).add_focus_changed_event_handler)(
                self.automation.as_ptr(),
                core::ptr::null_mut(),
                handler.as_ptr().cast(),
            )
        };
        if !succeeded(result) {
            // SAFETY: registration failed, so the just-created handler still
            // has exactly its original owned reference.
            unsafe { release_handler(handler.as_ptr()) };
            return Err(UiAutomationError::Query(result));
        }
        Ok(UiAutomationFocusSubscription {
            client: self,
            handler,
            state,
        })
    }
}

#[derive(Default)]
struct FocusEventState {
    armed: AtomicBool,
    outcome: Mutex<Option<Result<String, UiAutomationError>>>,
    ready: Condvar,
}

impl FocusEventState {
    fn arm(&self) {
        self.armed.store(false, Ordering::Release);
        let mut outcome = self
            .outcome
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *outcome = None;
        self.armed.store(true, Ordering::Release);
    }

    fn disarm(&self) {
        self.armed.store(false, Ordering::Release);
    }

    fn complete_sender(&self, sender: *mut raw::Element) {
        self.complete(text_property_from_raw(
            sender,
            raw::UIA_AUTOMATION_ID_PROPERTY_ID,
        ));
    }

    fn complete(&self, outcome: Result<String, UiAutomationError>) {
        if !self.armed.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut recorded = self
            .outcome
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *recorded = Some(outcome);
        self.ready.notify_one();
    }

    fn wait_for_automation_id(&self, timeout: Duration) -> Result<String, UiAutomationError> {
        let outcome = self
            .outcome
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let (mut outcome, _) = self
            .ready
            .wait_timeout_while(outcome, timeout, |current| current.is_none())
            .unwrap_or_else(|poison| poison.into_inner());
        self.disarm();
        let Some(outcome) = outcome.take() else {
            return Err(UiAutomationError::UnexpectedTree);
        };
        outcome
    }
}

#[repr(C)]
struct FocusEventHandler {
    vtable: *const FocusEventHandlerVtable,
    references: AtomicU32,
    state: Arc<FocusEventState>,
}

impl FocusEventHandler {
    fn new(state: Arc<FocusEventState>) -> NonNull<Self> {
        let handler = Box::new(Self {
            vtable: &FOCUS_EVENT_HANDLER_VTABLE,
            references: AtomicU32::new(1),
            state,
        });
        // SAFETY: Box allocations are non-null and become owned by COM-style
        // reference counting until `release_handler` reaches zero.
        unsafe { NonNull::new_unchecked(Box::into_raw(handler)) }
    }
}

#[repr(C)]
struct FocusEventHandlerVtable {
    query_interface: unsafe extern "system" fn(
        *mut FocusEventHandler,
        *const raw::Guid,
        *mut *mut c_void,
    ) -> raw::Hresult,
    add_ref: unsafe extern "system" fn(*mut FocusEventHandler) -> u32,
    release: unsafe extern "system" fn(*mut FocusEventHandler) -> u32,
    handle_focus_changed_event:
        unsafe extern "system" fn(*mut FocusEventHandler, *mut raw::Element) -> raw::Hresult,
}

static FOCUS_EVENT_HANDLER_VTABLE: FocusEventHandlerVtable = FocusEventHandlerVtable {
    query_interface,
    add_ref,
    release,
    handle_focus_changed_event,
};

unsafe extern "system" fn query_interface(
    this: *mut FocusEventHandler,
    interface: *const raw::Guid,
    output: *mut *mut c_void,
) -> raw::Hresult {
    if output.is_null() || interface.is_null() || this.is_null() {
        return raw::E_POINTER;
    }
    // SAFETY: `output` was checked for null and always starts cleared for a
    // refused query, matching the COM out-pointer contract.
    unsafe { *output = core::ptr::null_mut() };
    // SAFETY: `interface` was checked for null and names the requested IID.
    let accepted = unsafe {
        *interface == raw::IID_I_UNKNOWN
            || *interface == raw::IID_I_UI_AUTOMATION_FOCUS_CHANGED_EVENT_HANDLER
    };
    if !accepted {
        return raw::E_NOINTERFACE;
    }
    // SAFETY: `this` is the live callback object Windows queried. Incrementing
    // before publishing the pointer grants the caller one owned COM reference.
    unsafe {
        add_ref(this);
        *output = this.cast();
    }
    raw::S_OK
}

unsafe extern "system" fn add_ref(this: *mut FocusEventHandler) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: a non-null callback pointer names the reference counter owned by
    // this COM object. Saturation is defensive; a real callback cannot hold
    // more than `u32::MAX` references.
    unsafe {
        (*this)
            .references
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_add(1)
            })
    }
    .map_or(u32::MAX, |count| count + 1)
}

unsafe extern "system" fn release(this: *mut FocusEventHandler) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: a non-null callback pointer names this object's one reference
    // counter. A zero count is an invalid duplicate Release and is ignored.
    let remaining = unsafe {
        (*this)
            .references
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_sub(1)
            })
            .ok()
            .map(|count| count - 1)
    };
    let Some(remaining) = remaining else {
        return 0;
    };
    if remaining == 0 {
        // SAFETY: this call consumed the final COM reference, so no future
        // access may use the allocation.
        unsafe { drop(Box::from_raw(this)) };
    }
    remaining
}

unsafe fn release_handler(handler: *mut FocusEventHandler) {
    // SAFETY: callers own exactly one callback reference that this function
    // releases through the same vtable operation Windows receives.
    unsafe { release(handler) };
}

unsafe extern "system" fn handle_focus_changed_event(
    this: *mut FocusEventHandler,
    sender: *mut raw::Element,
) -> raw::Hresult {
    if this.is_null() {
        return raw::E_POINTER;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `this` is a live callback pointer while Windows dispatches
        // this method. The state uses synchronization because dispatch may be
        // on a different COM worker thread.
        let state = unsafe { &(*this).state };
        if !state.armed.load(Ordering::Acquire) {
            return;
        }
        state.complete_sender(sender);
    }));
    if outcome.is_ok() {
        raw::S_OK
    } else {
        raw::E_FAIL
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::FocusEventState;

    #[test]
    fn armed_state_accepts_one_sender_identifier() {
        let state = FocusEventState::default();
        state.arm();
        state.complete(Ok("ui.lab.field".to_owned()));
        assert_eq!(
            state
                .wait_for_automation_id(Duration::ZERO)
                .expect("the synthetic sender identifier is recorded"),
            "ui.lab.field"
        );
    }

    #[test]
    fn unarmed_state_discards_a_callback() {
        let state = FocusEventState::default();
        state.complete(Ok("ui.lab.field".to_owned()));
        assert!(state.wait_for_automation_id(Duration::ZERO).is_err());
    }

    #[test]
    fn callback_failures_reach_the_waiting_worker() {
        let state = FocusEventState::default();
        state.arm();
        state.complete(Err(super::UiAutomationError::PropertyType));
        assert!(state.wait_for_automation_id(Duration::ZERO).is_err());
    }
}
