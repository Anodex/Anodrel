//! Exact COM ABI for the direct Windows taskbar progress surface.

use std::{ffi::c_void, ptr};

use crate::TaskbarProgress;

type Hresult = i32;
type Hwnd = isize;

const COINIT_APARTMENTTHREADED: u32 = 0x2;
const CLSCTX_INPROC_SERVER: u32 = 0x1;
const RPC_E_CHANGED_MODE: Hresult = 0x8001_0106_u32 as Hresult;
const TBPF_NOPROGRESS: u32 = 0;
const TBPF_INDETERMINATE: u32 = 1;
const TBPF_NORMAL: u32 = 2;

/// `CLSID_TaskbarList`, supplied by Windows rather than a third party.
const CLSID_TASKBAR_LIST: Guid = Guid::new(
    0x56FDF344,
    0xFD6D,
    0x11D0,
    [0x95, 0x8A, 0x00, 0x60, 0x97, 0xC9, 0xA0, 0x90],
);
/// `IID_ITaskbarList3` from the Windows Shell SDK contract.
const IID_TASKBAR_LIST_3: Guid = Guid::new(
    0xEA1AFB91,
    0x9E28,
    0x4B86,
    [0x90, 0xE9, 0x9E, 0x9F, 0x8A, 0x5E, 0xEF, 0xAF],
);

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

impl Guid {
    const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}

#[repr(C)]
struct TaskbarList3 {
    vtable: *const TaskbarList3Vtable,
}

#[repr(C)]
struct TaskbarList3Vtable {
    _query_interface: usize,
    _add_ref: usize,
    release: unsafe extern "system" fn(*mut TaskbarList3) -> u32,
    initialize: unsafe extern "system" fn(*mut TaskbarList3) -> Hresult,
    // ITaskbarList adds four entries after HrInit and ITaskbarList2 adds one.
    // SetProgressValue and SetProgressState are then the first two
    // ITaskbarList3 entries, at their documented inherited ABI positions.
    _inherited: [usize; 5],
    set_progress_value: unsafe extern "system" fn(*mut TaskbarList3, Hwnd, u64, u64) -> Hresult,
    set_progress_state: unsafe extern "system" fn(*mut TaskbarList3, Hwnd, u32) -> Hresult,
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> Hresult;
    fn CoUninitialize();
    fn CoCreateInstance(
        class_id: *const Guid,
        outer: *mut c_void,
        context: u32,
        interface_id: *const Guid,
        object: *mut *mut c_void,
    ) -> Hresult;
}

/// Applies one state using a short-lived UI-thread COM object.
pub(crate) fn set(window: Hwnd, progress: TaskbarProgress) -> bool {
    let Some(_apartment) = ComApartment::enter() else {
        return false;
    };
    let Some(taskbar) = TaskbarList::create() else {
        return false;
    };
    if !succeeded(taskbar.initialize()) {
        return false;
    }
    match progress {
        TaskbarProgress::Clear => succeeded(taskbar.set_state(window, TBPF_NOPROGRESS)),
        TaskbarProgress::Activity => succeeded(taskbar.set_state(window, TBPF_INDETERMINATE)),
        TaskbarProgress::Determinate { completed, total } => {
            succeeded(taskbar.set_value(window, completed, total))
                && succeeded(taskbar.set_state(window, TBPF_NORMAL))
        }
    }
}

fn succeeded(result: Hresult) -> bool {
    result >= 0
}

/// Balances only an apartment initialization this call successfully performed.
struct ComApartment {
    uninitialize: bool,
}

impl ComApartment {
    fn enter() -> Option<Self> {
        // SAFETY: this thread performs a conventional apartment initialization;
        // a successful call is paired in Drop on this same thread.
        let result = unsafe { CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED) };
        if succeeded(result) {
            Some(Self { uninitialize: true })
        } else if result == RPC_E_CHANGED_MODE {
            // Another host component selected this UI thread's apartment. It
            // remains initialized, so this adapter must not uninitialize it.
            Some(Self {
                uninitialize: false,
            })
        } else {
            None
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.uninitialize {
            // SAFETY: this exact thread owns the successful matching call.
            unsafe { CoUninitialize() };
        }
    }
}

/// One owned direct `ITaskbarList3` pointer.
struct TaskbarList(*mut TaskbarList3);

impl TaskbarList {
    fn create() -> Option<Self> {
        let mut object = ptr::null_mut();
        // SAFETY: both GUIDs are immutable ABI values, aggregation is denied,
        // and object is writable storage for exactly one COM interface pointer.
        let result = unsafe {
            CoCreateInstance(
                &CLSID_TASKBAR_LIST,
                ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_TASKBAR_LIST_3,
                &mut object,
            )
        };
        (succeeded(result) && !object.is_null()).then_some(Self(object.cast()))
    }

    fn initialize(&self) -> Hresult {
        // SAFETY: self owns a live ITaskbarList3 and this invokes its inherited
        // ITaskbarList::HrInit slot before any progress operation.
        unsafe { ((*(*self.0).vtable).initialize)(self.0) }
    }

    fn set_value(&self, window: Hwnd, completed: u64, total: u64) -> Hresult {
        // SAFETY: the object is initialized and live; window belongs to the
        // caller's UI thread, and completed/total are validated by the facade.
        unsafe { ((*(*self.0).vtable).set_progress_value)(self.0, window, completed, total) }
    }

    fn set_state(&self, window: Hwnd, state: u32) -> Hresult {
        // SAFETY: the object is initialized and live; state is one fixed
        // TBPFLAG and window belongs to the caller's UI thread.
        unsafe { ((*(*self.0).vtable).set_progress_state)(self.0, window, state) }
    }
}

impl Drop for TaskbarList {
    fn drop(&mut self) {
        // SAFETY: CoCreateInstance gave this wrapper one owned interface
        // reference. Its Release slot is the standard IUnknown third entry.
        unsafe {
            let _ = ((*(*self.0).vtable).release)(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IID_TASKBAR_LIST_3, TaskbarList3Vtable};

    #[test]
    fn taskbar_vtable_preserves_the_inherited_progress_slots() {
        assert_eq!(
            std::mem::size_of::<TaskbarList3Vtable>(),
            11 * size_of::<usize>()
        );
        assert_eq!(IID_TASKBAR_LIST_3.data1, 0xEA1AFB91);
        assert_eq!(IID_TASKBAR_LIST_3.data2, 0x9E28);
        assert_eq!(IID_TASKBAR_LIST_3.data3, 0x4B86);
        assert_eq!(
            IID_TASKBAR_LIST_3.data4,
            [0x90, 0xE9, 0x9E, 0x9F, 0x8A, 0x5E, 0xEF, 0xAF]
        );
    }
}
