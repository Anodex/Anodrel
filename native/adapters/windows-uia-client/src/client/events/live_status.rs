//! One private live-status listener for host acceptance diagnostics.
//!
//! Production code never listens to its own UI Automation events. This direct
//! client owns one short-lived callback only while a fixed host probe verifies
//! that Windows delivered one `LiveRegionChanged` notification.

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

use super::super::{
    UiAutomationClient, UiAutomationElement, UiAutomationError, text_property_from_raw,
};

/// One short-lived live-status listener owned by a host acceptance diagnostic.
///
/// The caller arms one observation under one host-selected root, invokes one
/// fixed action, and waits for one compiled status ID. No application can
/// select an event, root, listener, sender, or result.
pub struct UiAutomationLiveStatusSubscription<'automation, 'element> {
    client: &'automation UiAutomationClient,
    root: &'element UiAutomationElement,
    handler: NonNull<LiveStatusEventHandler>,
    state: Arc<LiveStatusEventState>,
}

impl UiAutomationLiveStatusSubscription<'_, '_> {
    /// Starts one fresh private live-status observation.
    pub fn arm(&self) {
        self.state.arm();
    }

    /// Waits for one fixed status source and live-region event ID.
    pub fn wait_for_status(
        &self,
        expected_status: &str,
        timeout: Duration,
    ) -> Result<(), UiAutomationError> {
        let Some(observed) = self.state.wait_for_event(timeout)? else {
            return Err(UiAutomationError::EventNotObserved);
        };
        if observed.sender_automation_id == expected_status
            && observed.event_id == raw::UIA_LIVE_REGION_CHANGED_EVENT_ID
        {
            Ok(())
        } else {
            Err(UiAutomationError::UnexpectedTree)
        }
    }
}

impl Drop for UiAutomationLiveStatusSubscription<'_, '_> {
    fn drop(&mut self) {
        self.state.disarm();
        // SAFETY: `client`, `root`, and `handler` remain live for this guard's
        // whole lifetime. Removal ends Windows' registration before this guard
        // releases its own callback reference.
        unsafe {
            let vtable = (*self.client.automation.as_ptr()).vtable;
            let _ = ((*vtable).remove_automation_event_handler)(
                self.client.automation.as_ptr(),
                raw::UIA_LIVE_REGION_CHANGED_EVENT_ID,
                self.root.raw.as_ptr(),
                self.handler.as_ptr().cast(),
            );
            release_handler(self.handler.as_ptr());
        }
    }
}

impl UiAutomationClient {
    /// Registers one private live-status callback below a fixed root.
    ///
    /// Subtree scope is necessary because the provider raises the event from
    /// the status child, while the host diagnostic holds only its fixed root.
    pub fn subscribe_to_live_status_changes<'element>(
        &self,
        root: &'element UiAutomationElement,
    ) -> Result<UiAutomationLiveStatusSubscription<'_, 'element>, UiAutomationError> {
        let state = Arc::new(LiveStatusEventState::default());
        let handler = LiveStatusEventHandler::new(Arc::clone(&state));
        // SAFETY: `automation`, `root`, and the handler's first field are live
        // COM interfaces. The fixed event and subtree scope have no caller
        // input, and a null cache request retains no property cache.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).add_automation_event_handler)(
                self.automation.as_ptr(),
                raw::UIA_LIVE_REGION_CHANGED_EVENT_ID,
                root.raw.as_ptr(),
                raw::TREE_SCOPE_SUBTREE,
                core::ptr::null_mut(),
                handler.as_ptr().cast(),
            )
        };
        if !succeeded(result) {
            // SAFETY: registration failed, so the handler retains only its
            // original reference and may be released immediately.
            unsafe { release_handler(handler.as_ptr()) };
            return Err(UiAutomationError::Query(result));
        }
        Ok(UiAutomationLiveStatusSubscription {
            client: self,
            root,
            handler,
            state,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LiveStatusEvent {
    sender_automation_id: String,
    event_id: i32,
}

#[derive(Default)]
struct LiveStatusEventState {
    armed: AtomicBool,
    outcome: Mutex<Option<Result<LiveStatusEvent, UiAutomationError>>>,
    ready: Condvar,
}

impl LiveStatusEventState {
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

    fn complete_sender(&self, sender: *mut raw::Element, event_id: i32) {
        self.complete(
            text_property_from_raw(sender, raw::UIA_AUTOMATION_ID_PROPERTY_ID).map(
                |sender_automation_id| LiveStatusEvent {
                    sender_automation_id,
                    event_id,
                },
            ),
        );
    }

    fn complete(&self, outcome: Result<LiveStatusEvent, UiAutomationError>) {
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

    fn wait_for_event(
        &self,
        timeout: Duration,
    ) -> Result<Option<LiveStatusEvent>, UiAutomationError> {
        let outcome = self
            .outcome
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let (mut outcome, _) = self
            .ready
            .wait_timeout_while(outcome, timeout, |current| current.is_none())
            .unwrap_or_else(|poison| poison.into_inner());
        self.disarm();
        outcome.take().transpose()
    }
}

#[repr(C)]
struct LiveStatusEventHandler {
    vtable: *const LiveStatusEventHandlerVtable,
    references: AtomicU32,
    state: Arc<LiveStatusEventState>,
}

impl LiveStatusEventHandler {
    fn new(state: Arc<LiveStatusEventState>) -> NonNull<Self> {
        let handler = Box::new(Self {
            vtable: &LIVE_STATUS_EVENT_HANDLER_VTABLE,
            references: AtomicU32::new(1),
            state,
        });
        // SAFETY: Box allocations are non-null and become owned by COM-style
        // reference counting until `release_handler` reaches zero.
        unsafe { NonNull::new_unchecked(Box::into_raw(handler)) }
    }
}

#[repr(C)]
struct LiveStatusEventHandlerVtable {
    query_interface: unsafe extern "system" fn(
        *mut LiveStatusEventHandler,
        *const raw::Guid,
        *mut *mut c_void,
    ) -> raw::Hresult,
    add_ref: unsafe extern "system" fn(*mut LiveStatusEventHandler) -> u32,
    release: unsafe extern "system" fn(*mut LiveStatusEventHandler) -> u32,
    handle_automation_event: unsafe extern "system" fn(
        *mut LiveStatusEventHandler,
        *mut raw::Element,
        i32,
    ) -> raw::Hresult,
}

static LIVE_STATUS_EVENT_HANDLER_VTABLE: LiveStatusEventHandlerVtable =
    LiveStatusEventHandlerVtable {
        query_interface,
        add_ref,
        release,
        handle_automation_event,
    };

unsafe extern "system" fn query_interface(
    this: *mut LiveStatusEventHandler,
    interface: *const raw::Guid,
    output: *mut *mut c_void,
) -> raw::Hresult {
    if output.is_null() || interface.is_null() || this.is_null() {
        return raw::E_POINTER;
    }
    // SAFETY: `output` was checked for null and starts cleared for refusal.
    unsafe { *output = core::ptr::null_mut() };
    // SAFETY: `interface` was checked for null and names the requested IID.
    let accepted = unsafe {
        *interface == raw::IID_I_UNKNOWN || *interface == raw::IID_I_UI_AUTOMATION_EVENT_HANDLER
    };
    if !accepted {
        return raw::E_NOINTERFACE;
    }
    // SAFETY: `this` is live while Windows queries it. The new reference is
    // granted before the callback pointer is published.
    unsafe {
        add_ref(this);
        *output = this.cast();
    }
    raw::S_OK
}

unsafe extern "system" fn add_ref(this: *mut LiveStatusEventHandler) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: a non-null callback pointer names this object's reference count.
    unsafe {
        (*this)
            .references
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_add(1)
            })
    }
    .map_or(u32::MAX, |count| count + 1)
}

unsafe extern "system" fn release(this: *mut LiveStatusEventHandler) -> u32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: a non-null callback pointer names this object's reference count.
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
        // SAFETY: this operation consumed the final COM reference.
        unsafe { drop(Box::from_raw(this)) };
    }
    remaining
}

unsafe fn release_handler(handler: *mut LiveStatusEventHandler) {
    // SAFETY: callers own one original callback reference.
    unsafe { release(handler) };
}

unsafe extern "system" fn handle_automation_event(
    this: *mut LiveStatusEventHandler,
    sender: *mut raw::Element,
    event_id: i32,
) -> raw::Hresult {
    if this.is_null() {
        return raw::E_POINTER;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Windows calls this while `this` is live. The state is
        // synchronized because event dispatch can use a different COM thread.
        let state = unsafe { &(*this).state };
        if state.armed.load(Ordering::Acquire) {
            state.complete_sender(sender, event_id);
        }
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

    use super::{LiveStatusEvent, LiveStatusEventState};

    #[test]
    fn armed_state_records_one_live_status_event() {
        let state = LiveStatusEventState::default();
        state.arm();
        state.complete(Ok(LiveStatusEvent {
            sender_automation_id: "native.live.status".to_owned(),
            event_id: 20_024,
        }));
        assert_eq!(
            state
                .wait_for_event(Duration::ZERO)
                .expect("the synthetic event is recorded"),
            Some(LiveStatusEvent {
                sender_automation_id: "native.live.status".to_owned(),
                event_id: 20_024,
            })
        );
    }

    #[test]
    fn unarmed_state_discards_a_callback() {
        let state = LiveStatusEventState::default();
        state.complete(Ok(LiveStatusEvent {
            sender_automation_id: "native.live.status".to_owned(),
            event_id: 20_024,
        }));
        assert_eq!(
            state
                .wait_for_event(Duration::ZERO)
                .expect("an unarmed event is discarded"),
            None
        );
    }

    #[test]
    fn callback_failures_reach_the_waiting_worker() {
        let state = LiveStatusEventState::default();
        state.arm();
        state.complete(Err(super::UiAutomationError::PropertyType));
        assert!(state.wait_for_event(Duration::ZERO).is_err());
    }
}
