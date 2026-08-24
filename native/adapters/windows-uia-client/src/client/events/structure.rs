//! One private structure-event subscription for host acceptance diagnostics.
//!
//! Production code never subscribes to its own accessibility events. This
//! direct client owns one short-lived callback only while a host diagnostic
//! verifies Windows delivered a fixed `ChildrenInvalidated` notification.

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

/// One short-lived structure-event listener owned by a host acceptance check.
///
/// The caller arms exactly one observation for a host-selected root, then
/// triggers its own fixed document replacement. No application can choose the
/// root, listener, event type, sender, or result.
pub struct UiAutomationStructureSubscription<'automation, 'element> {
    client: &'automation UiAutomationClient,
    root: &'element UiAutomationElement,
    handler: NonNull<StructureEventHandler>,
    state: Arc<StructureEventState>,
}

impl UiAutomationStructureSubscription<'_, '_> {
    /// Starts one fresh private structure-event observation.
    pub fn arm(&self) {
        self.state.arm();
    }

    /// Waits for one fixed root `ChildrenInvalidated` notification.
    ///
    /// The handler records only the source AutomationId and closed Windows
    /// change kind. The callback's runtime-ID representation belongs to Windows
    /// and is neither read nor used to infer the provider call's input.
    /// Nothing becomes application, protocol, or SDK data.
    pub fn wait_for_children_invalidated(
        &self,
        expected_root: &str,
        timeout: Duration,
    ) -> Result<(), UiAutomationError> {
        let Some(observed) = self.state.wait_for_event(timeout)? else {
            return Err(UiAutomationError::EventNotObserved);
        };
        if observed.sender_automation_id == expected_root
            && observed.change_type == raw::STRUCTURE_CHANGE_CHILDREN_INVALIDATED
        {
            Ok(())
        } else {
            Err(UiAutomationError::UnexpectedTree)
        }
    }
}

impl Drop for UiAutomationStructureSubscription<'_, '_> {
    fn drop(&mut self) {
        self.state.disarm();
        // SAFETY: `client`, `root`, and `handler` remain live for this guard's
        // whole lifetime. Removal ends Windows' registration before this guard
        // releases its own callback reference. Drop cannot report a failed
        // best-effort removal, and Windows still owns any reference it kept.
        unsafe {
            let vtable = (*self.client.automation.as_ptr()).vtable;
            let _ = ((*vtable).remove_structure_changed_event_handler)(
                self.client.automation.as_ptr(),
                self.root.raw.as_ptr(),
                self.handler.as_ptr().cast(),
            );
            release_handler(self.handler.as_ptr());
        }
    }
}

impl UiAutomationClient {
    /// Registers one private root structure-event callback for a host check.
    ///
    /// The element scope intentionally observes only the supplied fixed root,
    /// not its children or any other window. The callback copies only its
    /// closed verification values into one bounded local result slot.
    pub fn subscribe_to_structure_changes<'element>(
        &self,
        root: &'element UiAutomationElement,
    ) -> Result<UiAutomationStructureSubscription<'_, 'element>, UiAutomationError> {
        let state = Arc::new(StructureEventState::default());
        let handler = StructureEventHandler::new(Arc::clone(&state));
        // SAFETY: `automation`, `root`, and the handler's first field are live
        // COM interfaces. Element scope selects only the fixed root, a null
        // cache request asks for no stored property cache, and this guard owns
        // the handler's original reference.
        let result = unsafe {
            let vtable = (*self.automation.as_ptr()).vtable;
            ((*vtable).add_structure_changed_event_handler)(
                self.automation.as_ptr(),
                root.raw.as_ptr(),
                raw::TREE_SCOPE_ELEMENT,
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
        Ok(UiAutomationStructureSubscription {
            client: self,
            root,
            handler,
            state,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StructureEvent {
    sender_automation_id: String,
    change_type: i32,
}

#[derive(Default)]
struct StructureEventState {
    armed: AtomicBool,
    outcome: Mutex<Option<Result<StructureEvent, UiAutomationError>>>,
    ready: Condvar,
}

impl StructureEventState {
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

    fn complete_sender(
        &self,
        sender: *mut raw::Element,
        change_type: i32,
        _runtime_id: *mut c_void,
    ) {
        self.complete(
            text_property_from_raw(sender, raw::UIA_AUTOMATION_ID_PROPERTY_ID).map(
                |sender_automation_id| StructureEvent {
                    sender_automation_id,
                    change_type,
                },
            ),
        );
    }

    fn complete(&self, outcome: Result<StructureEvent, UiAutomationError>) {
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
    ) -> Result<Option<StructureEvent>, UiAutomationError> {
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
struct StructureEventHandler {
    vtable: *const StructureEventHandlerVtable,
    references: AtomicU32,
    state: Arc<StructureEventState>,
}

impl StructureEventHandler {
    fn new(state: Arc<StructureEventState>) -> NonNull<Self> {
        let handler = Box::new(Self {
            vtable: &STRUCTURE_EVENT_HANDLER_VTABLE,
            references: AtomicU32::new(1),
            state,
        });
        // SAFETY: Box allocations are non-null and become owned by COM-style
        // reference counting until `release_handler` reaches zero.
        unsafe { NonNull::new_unchecked(Box::into_raw(handler)) }
    }
}

#[repr(C)]
struct StructureEventHandlerVtable {
    query_interface: unsafe extern "system" fn(
        *mut StructureEventHandler,
        *const raw::Guid,
        *mut *mut c_void,
    ) -> raw::Hresult,
    add_ref: unsafe extern "system" fn(*mut StructureEventHandler) -> u32,
    release: unsafe extern "system" fn(*mut StructureEventHandler) -> u32,
    handle_structure_changed_event: unsafe extern "system" fn(
        *mut StructureEventHandler,
        *mut raw::Element,
        i32,
        *mut c_void,
    ) -> raw::Hresult,
}

static STRUCTURE_EVENT_HANDLER_VTABLE: StructureEventHandlerVtable = StructureEventHandlerVtable {
    query_interface,
    add_ref,
    release,
    handle_structure_changed_event,
};

unsafe extern "system" fn query_interface(
    this: *mut StructureEventHandler,
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
            || *interface == raw::IID_I_UI_AUTOMATION_STRUCTURE_CHANGED_EVENT_HANDLER
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

unsafe extern "system" fn add_ref(this: *mut StructureEventHandler) -> u32 {
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

unsafe extern "system" fn release(this: *mut StructureEventHandler) -> u32 {
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

unsafe fn release_handler(handler: *mut StructureEventHandler) {
    // SAFETY: callers own exactly one callback reference that this function
    // releases through the same vtable operation Windows receives.
    unsafe { release(handler) };
}

unsafe extern "system" fn handle_structure_changed_event(
    this: *mut StructureEventHandler,
    sender: *mut raw::Element,
    change_type: i32,
    runtime_id: *mut c_void,
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
        state.complete_sender(sender, change_type, runtime_id);
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

    use super::{StructureEvent, StructureEventState};

    #[test]
    fn armed_state_records_one_structure_event() {
        let state = StructureEventState::default();
        state.arm();
        state.complete(Ok(StructureEvent {
            sender_automation_id: "anodrel.surface".to_owned(),
            change_type: 2,
        }));
        assert_eq!(
            state
                .wait_for_event(Duration::ZERO)
                .expect("the synthetic structure event is recorded"),
            Some(StructureEvent {
                sender_automation_id: "anodrel.surface".to_owned(),
                change_type: 2,
            })
        );
    }

    #[test]
    fn unarmed_state_discards_a_callback() {
        let state = StructureEventState::default();
        state.complete(Ok(StructureEvent {
            sender_automation_id: "anodrel.surface".to_owned(),
            change_type: 2,
        }));
        assert_eq!(
            state
                .wait_for_event(Duration::ZERO)
                .expect("an unarmed callback must not become an event"),
            None
        );
    }

    #[test]
    fn callback_failures_reach_the_waiting_worker() {
        let state = StructureEventState::default();
        state.arm();
        state.complete(Err(super::UiAutomationError::PropertyType));
        assert!(state.wait_for_event(Duration::ZERO).is_err());
    }
}
