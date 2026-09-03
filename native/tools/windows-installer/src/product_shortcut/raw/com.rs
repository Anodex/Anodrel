//! COM apartment, lifetime, and Shell Link ABI for product registration.

use std::{
    ffi::c_void,
    path::Path,
    ptr::{self, NonNull},
};

use super::{ShortcutWriteError, wide_path};

type Hresult = i32;

const COINIT_MULTITHREADED: u32 = 0;
const CLSCTX_INPROC_SERVER: u32 = 1;

const CLSID_SHELL_LINK: Guid = Guid::new(0x0002_1401, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_I_SHELL_LINK_W: Guid = Guid::new(0x0002_14f9, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_I_PERSIST_FILE: Guid = Guid::new(0x0000_010b, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);

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
struct Unknown {
    vtable: *const UnknownVtable,
}

#[repr(C)]
struct UnknownVtable {
    query_interface:
        unsafe extern "system" fn(*mut Unknown, *const Guid, *mut *mut c_void) -> Hresult,
    add_ref: *const c_void,
    release: unsafe extern "system" fn(*mut Unknown) -> u32,
}

#[repr(C)]
struct ShellLink {
    vtable: *const ShellLinkVtable,
}

/// The documented `IShellLinkW` layout through the two methods this adapter calls.
#[repr(C)]
struct ShellLinkVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: *const c_void,
    get_path: *const c_void,
    get_id_list: *const c_void,
    set_id_list: *const c_void,
    get_description: *const c_void,
    set_description: *const c_void,
    get_working_directory: *const c_void,
    set_working_directory: unsafe extern "system" fn(*mut ShellLink, *const u16) -> Hresult,
    get_arguments: unsafe extern "system" fn(*mut ShellLink, *mut u16, i32) -> Hresult,
    set_arguments: unsafe extern "system" fn(*mut ShellLink, *const u16) -> Hresult,
    get_hotkey: *const c_void,
    set_hotkey: *const c_void,
    get_show_cmd: *const c_void,
    set_show_cmd: *const c_void,
    get_icon_location: *const c_void,
    set_icon_location: *const c_void,
    set_relative_path: *const c_void,
    resolve: *const c_void,
    set_path: unsafe extern "system" fn(*mut ShellLink, *const u16) -> Hresult,
}

#[repr(C)]
struct PersistFile {
    vtable: *const PersistFileVtable,
}

/// The documented `IPersistFile` layout through `Save`.
#[repr(C)]
struct PersistFileVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: *const c_void,
    get_class_id: *const c_void,
    is_dirty: *const c_void,
    load: unsafe extern "system" fn(*mut PersistFile, *const u16, u32) -> Hresult,
    save: unsafe extern "system" fn(*mut PersistFile, *const u16, i32) -> Hresult,
    save_completed: *const c_void,
    get_cur_file: *const c_void,
}

#[link(name = "Ole32")]
unsafe extern "system" {
    fn CoCreateInstance(
        class: *const Guid,
        outer: *mut c_void,
        class_context: u32,
        interface: *const Guid,
        result: *mut *mut c_void,
    ) -> Hresult;
    fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> Hresult;
    fn CoUninitialize();
}

/// Persists one fixed staged Shell Link from already verified private paths.
pub(super) fn persist_link(
    launcher_path: &Path,
    package_root: &Path,
    arguments: &str,
    temporary_path: &Path,
) -> Result<(), ShortcutWriteError> {
    let _apartment = ComApartment::initialize()?;
    let link = create_shell_link()?;
    set_link_path(&link, launcher_path)?;
    set_link_working_directory(&link, package_root)?;
    set_link_arguments(&link, arguments)?;
    let persistence = query_persist_file(&link)?;
    let temporary_path = wide_path(temporary_path)?;
    // SAFETY: `persistence` owns a valid `IPersistFile`; the staged path is
    // NUL terminated, within the fixed normal parent, and `1` is TRUE for the
    // documented remember-path flag.
    let result = unsafe {
        ((*(*persistence.as_ptr()).vtable).save)(persistence.as_ptr(), temporary_path.as_ptr(), 1)
    };
    succeeded(result)
        .then_some(())
        .ok_or(ShortcutWriteError::LinkSaveFailed)
}

fn set_link_arguments(link: &Com<ShellLink>, arguments: &str) -> Result<(), ShortcutWriteError> {
    let arguments = wide_text(arguments)?;
    // SAFETY: `link` owns an `IShellLinkW`; the fixed generated argument text
    // is NUL terminated for this synchronous documented call.
    let result =
        unsafe { ((*(*link.as_ptr()).vtable).set_arguments)(link.as_ptr(), arguments.as_ptr()) };
    succeeded(result)
        .then_some(())
        .ok_or(ShortcutWriteError::LinkSaveFailed)
}

fn create_shell_link() -> Result<Com<ShellLink>, ShortcutWriteError> {
    let mut raw_link = ptr::null_mut();
    // SAFETY: both GUIDs name documented Shell Link types; aggregation is
    // absent; and `raw_link` is writable out-parameter storage.
    let result = unsafe {
        CoCreateInstance(
            &CLSID_SHELL_LINK,
            ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_I_SHELL_LINK_W,
            &mut raw_link,
        )
    };
    if !succeeded(result) {
        return Err(ShortcutWriteError::ComUnavailable);
    }
    Com::from_out(raw_link.cast::<ShellLink>())
}

fn set_link_path(link: &Com<ShellLink>, path: &Path) -> Result<(), ShortcutWriteError> {
    let path = wide_path(path)?;
    // SAFETY: `link` owns an `IShellLinkW`; `path` remains NUL terminated for
    // the documented synchronous call.
    let result = unsafe { ((*(*link.as_ptr()).vtable).set_path)(link.as_ptr(), path.as_ptr()) };
    succeeded(result)
        .then_some(())
        .ok_or(ShortcutWriteError::LinkSaveFailed)
}

fn set_link_working_directory(
    link: &Com<ShellLink>,
    path: &Path,
) -> Result<(), ShortcutWriteError> {
    let path = wide_path(path)?;
    // SAFETY: `link` owns an `IShellLinkW`; `path` remains NUL terminated for
    // the documented synchronous call.
    let result =
        unsafe { ((*(*link.as_ptr()).vtable).set_working_directory)(link.as_ptr(), path.as_ptr()) };
    succeeded(result)
        .then_some(())
        .ok_or(ShortcutWriteError::LinkSaveFailed)
}

fn query_persist_file(link: &Com<ShellLink>) -> Result<Com<PersistFile>, ShortcutWriteError> {
    let unknown = link.as_ptr().cast::<Unknown>();
    let mut raw_persistence = ptr::null_mut();
    // SAFETY: every `IShellLinkW` starts with `IUnknown`; the fixed interface
    // GUID requests its documented `IPersistFile` implementation and the
    // writable out slot receives one reference on success.
    let result = unsafe {
        ((*(*unknown).vtable).query_interface)(unknown, &IID_I_PERSIST_FILE, &mut raw_persistence)
    };
    if !succeeded(result) {
        return Err(ShortcutWriteError::ComUnavailable);
    }
    Com::from_out(raw_persistence.cast::<PersistFile>())
}

fn wide_text(value: &str) -> Result<Vec<u16>, ShortcutWriteError> {
    if value.is_empty() || value.encode_utf16().any(|unit| unit == 0) {
        return Err(ShortcutWriteError::PathInvalid);
    }
    Ok(value.encode_utf16().chain(Some(0)).collect())
}

#[cfg(test)]
pub(super) fn read_persisted_arguments(path: &Path) -> Result<String, ShortcutWriteError> {
    let _apartment = ComApartment::initialize()?;
    let link = create_shell_link()?;
    let persistence = query_persist_file(&link)?;
    let path = wide_path(path)?;
    // SAFETY: `persistence` owns an `IPersistFile`; the test-created path is
    // NUL terminated and its mode is the documented read-only value.
    let loaded =
        unsafe { ((*(*persistence.as_ptr()).vtable).load)(persistence.as_ptr(), path.as_ptr(), 0) };
    if !succeeded(loaded) {
        return Err(ShortcutWriteError::LinkSaveFailed);
    }
    let mut buffer = [0_u16; 256];
    // SAFETY: `link` owns an `IShellLinkW` and `buffer` supplies a bounded
    // writable UTF-16 result range including a terminator slot.
    let read = unsafe {
        ((*(*link.as_ptr()).vtable).get_arguments)(
            link.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };
    if !succeeded(read) {
        return Err(ShortcutWriteError::LinkSaveFailed);
    }
    let length = buffer
        .iter()
        .position(|unit| *unit == 0)
        .ok_or(ShortcutWriteError::LinkSaveFailed)?;
    String::from_utf16(&buffer[..length]).map_err(|_| ShortcutWriteError::LinkSaveFailed)
}

fn succeeded(result: Hresult) -> bool {
    result >= 0
}

struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, ShortcutWriteError> {
        // SAFETY: the null reserved pointer and documented MTA flag are the
        // complete `CoInitializeEx` contract for this non-UI installer work.
        let result = unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED) };
        succeeded(result)
            .then_some(Self)
            .ok_or(ShortcutWriteError::ComUnavailable)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: construction succeeds only after this thread's successful
        // `CoInitializeEx`, which requires one balancing uninitialization.
        unsafe { CoUninitialize() };
    }
}

struct Com<T> {
    pointer: NonNull<T>,
}

impl<T> Com<T> {
    fn from_out(pointer: *mut T) -> Result<Self, ShortcutWriteError> {
        NonNull::new(pointer)
            .map(|pointer| Self { pointer })
            .ok_or(ShortcutWriteError::ComUnavailable)
    }

    const fn as_ptr(&self) -> *mut T {
        self.pointer.as_ptr()
    }
}

impl<T> Drop for Com<T> {
    fn drop(&mut self) {
        let unknown = self.pointer.as_ptr().cast::<Unknown>();
        // SAFETY: this type owns one non-null COM interface reference whose
        // vtable begins with the documented `IUnknown::Release` slot.
        unsafe {
            let vtable = (*unknown).vtable;
            ((*vtable).release)(unknown);
        }
    }
}
