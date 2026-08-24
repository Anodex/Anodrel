//! Narrow Common Item Dialog ABI for one filesystem-folder selection.

#![allow(non_snake_case)]

use core::{ffi::c_void, ptr::NonNull};
use std::slice;

type Hresult = i32;

const S_OK: Hresult = 0;
const COINIT_APARTMENTTHREADED: u32 = 0x2;
const CLSCTX_INPROC_SERVER: u32 = 0x1;
const HRESULT_CANCELLED: Hresult = 0x8007_04c7_u32 as i32;
const MAX_PATH_UNITS: usize = 32_768;

const FOS_OVERWRITEPROMPT: u32 = 0x0000_0002;
const FOS_STRICTFILETYPES: u32 = 0x0000_0004;
const FOS_PICKFOLDERS: u32 = 0x0000_0020;
const FOS_FORCEFILESYSTEM: u32 = 0x0000_0040;
const FOS_PATHMUSTEXIST: u32 = 0x0000_0800;
const FOS_FILEMUSTEXIST: u32 = 0x0000_1000;
const FOS_DONTADDTORECENT: u32 = 0x0200_0000;
const SIGDN_FILESYSPATH: u32 = 0x8005_8000;

/// The File Open Dialog COM class from `shobjidl_core.h`.
const CLSID_FILE_OPEN_DIALOG: Guid = Guid::new(
    0xdc1c_5a9c,
    0xe88a,
    0x4dde,
    [0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7],
);

/// The `IFileOpenDialog` interface from `shobjidl_core.h`.
const IID_I_FILE_OPEN_DIALOG: Guid = Guid::new(
    0xd57c_7288,
    0xd4ad,
    0x4768,
    [0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60],
);

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: unsafe extern "system" fn(*mut Unknown) -> u32,
}

#[repr(C)]
struct FileOpenDialog {
    vtable: *const FileOpenDialogVtable,
}

/// The inherited `IFileDialog` vtable prefix of `IFileOpenDialog`.
///
/// Unused slots remain pointer-sized placeholders so every invoked method is
/// at the exact documented Windows SDK offset.
#[repr(C)]
struct FileOpenDialogVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: *const c_void,
    show: unsafe extern "system" fn(*mut FileOpenDialog, isize) -> Hresult,
    set_file_types: *const c_void,
    set_file_type_index: *const c_void,
    get_file_type_index: *const c_void,
    advise: *const c_void,
    unadvise: *const c_void,
    set_options: unsafe extern "system" fn(*mut FileOpenDialog, u32) -> Hresult,
    get_options: unsafe extern "system" fn(*mut FileOpenDialog, *mut u32) -> Hresult,
    set_default_folder: *const c_void,
    set_folder: *const c_void,
    get_folder: *const c_void,
    get_current_selection: *const c_void,
    set_file_name: *const c_void,
    get_file_name: *const c_void,
    set_title: *const c_void,
    set_ok_button_label: *const c_void,
    set_file_name_label: *const c_void,
    get_result: unsafe extern "system" fn(*mut FileOpenDialog, *mut *mut ShellItem) -> Hresult,
}

#[repr(C)]
struct ShellItem {
    vtable: *const ShellItemVtable,
}

#[repr(C)]
struct ShellItemVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: *const c_void,
    bind_to_handler: *const c_void,
    get_parent: *const c_void,
    get_display_name: unsafe extern "system" fn(*mut ShellItem, u32, *mut *mut u16) -> Hresult,
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> Hresult;
    fn CoUninitialize();
    fn CoCreateInstance(
        class: *const Guid,
        outer: *mut c_void,
        class_context: u32,
        interface: *const Guid,
        result: *mut *mut c_void,
    ) -> Hresult;
    fn CoTaskMemFree(memory: *mut c_void);
}

/// Shows a modern, filesystem-only folder picker on the calling UI thread.
pub(super) fn select_folder(owner_window: isize) -> Result<Option<String>, ()> {
    let _apartment = ComApartment::initialize_sta()?;
    let dialog = create_dialog()?;
    configure_for_folder_selection(&dialog)?;
    if !show_dialog(&dialog, owner_window)? {
        return Ok(None);
    }
    let item = selected_shell_item(&dialog)?;
    shell_item_path(&item).map(Some)
}

struct ComApartment;

impl ComApartment {
    fn initialize_sta() -> Result<Self, ()> {
        // SAFETY: the null reserved pointer and documented STA flag are the
        // complete `CoInitializeEx` contract. A successful call is balanced
        // by this value's Drop on the same host UI thread.
        let result = unsafe { CoInitializeEx(core::ptr::null_mut(), COINIT_APARTMENTTHREADED) };
        if succeeded(result) { Ok(Self) } else { Err(()) }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: construction succeeds only after `CoInitializeEx` succeeds
        // on this same thread, which requires one balancing uninitialize.
        unsafe { CoUninitialize() };
    }
}

struct Com<T> {
    pointer: NonNull<T>,
}

impl<T> Com<T> {
    fn from_out(pointer: *mut T) -> Result<Self, ()> {
        NonNull::new(pointer)
            .map(|pointer| Self { pointer })
            .ok_or(())
    }

    const fn as_ptr(&self) -> *mut T {
        self.pointer.as_ptr()
    }
}

impl<T> Drop for Com<T> {
    fn drop(&mut self) {
        let unknown = self.pointer.as_ptr().cast::<Unknown>();
        // SAFETY: every owned COM interface begins with IUnknown; this final
        // `Release` balances the reference supplied by `CoCreateInstance` or
        // an out parameter.
        unsafe {
            let vtable = (*unknown).vtable;
            ((*vtable).release)(unknown);
        }
    }
}

fn create_dialog() -> Result<Com<FileOpenDialog>, ()> {
    let mut raw_dialog = core::ptr::null_mut();
    // SAFETY: both GUIDs identify documented shell dialog types, aggregation
    // is absent, and `raw_dialog` is writable out-parameter storage.
    let result = unsafe {
        CoCreateInstance(
            &CLSID_FILE_OPEN_DIALOG,
            core::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_I_FILE_OPEN_DIALOG,
            &mut raw_dialog,
        )
    };
    if !succeeded(result) {
        return Err(());
    }
    Com::from_out(raw_dialog.cast::<FileOpenDialog>())
}

fn configure_for_folder_selection(dialog: &Com<FileOpenDialog>) -> Result<(), ()> {
    // SAFETY: `dialog` owns a valid `IFileOpenDialog` with the documented
    // `IFileDialog` vtable prefix, and `current` is writable output storage.
    let mut current = 0;
    let result =
        unsafe { ((*(*dialog.as_ptr()).vtable).get_options)(dialog.as_ptr(), &mut current) };
    if !succeeded(result) {
        return Err(());
    }

    let options = folder_options(current);
    // SAFETY: `dialog` remains valid for the call and `options` contains only
    // documented `FILEOPENDIALOGOPTIONS` flags.
    let result = unsafe { ((*(*dialog.as_ptr()).vtable).set_options)(dialog.as_ptr(), options) };
    succeeded(result).then_some(()).ok_or(())
}

fn show_dialog(dialog: &Com<FileOpenDialog>, owner_window: isize) -> Result<bool, ()> {
    // SAFETY: `dialog` is valid and `owner_window` is supplied only by trusted
    // host window code as a native HWND-sized value.
    let result = unsafe { ((*(*dialog.as_ptr()).vtable).show)(dialog.as_ptr(), owner_window) };
    if result == HRESULT_CANCELLED {
        return Ok(false);
    }
    succeeded(result).then_some(true).ok_or(())
}

fn selected_shell_item(dialog: &Com<FileOpenDialog>) -> Result<Com<ShellItem>, ()> {
    let mut raw_item = core::ptr::null_mut();
    // SAFETY: `dialog` is a valid completed `IFileOpenDialog` and `raw_item`
    // is writable out-parameter storage for its selected `IShellItem`.
    let result =
        unsafe { ((*(*dialog.as_ptr()).vtable).get_result)(dialog.as_ptr(), &mut raw_item) };
    if !succeeded(result) {
        return Err(());
    }
    Com::from_out(raw_item)
}

fn shell_item_path(item: &Com<ShellItem>) -> Result<String, ()> {
    let mut raw_path = core::ptr::null_mut();
    // SAFETY: `item` owns a valid `IShellItem`, `SIGDN_FILESYSPATH` requests
    // its documented filesystem display name, and `raw_path` is writable
    // storage for the CoTaskMem allocation Windows returns.
    let result = unsafe {
        ((*(*item.as_ptr()).vtable).get_display_name)(
            item.as_ptr(),
            SIGDN_FILESYSPATH,
            &mut raw_path,
        )
    };
    if !succeeded(result) {
        return Err(());
    }
    let path = CoTaskMemWide::from_out(raw_path)?;
    path.to_string()
}

struct CoTaskMemWide {
    pointer: NonNull<u16>,
}

impl CoTaskMemWide {
    fn from_out(pointer: *mut u16) -> Result<Self, ()> {
        NonNull::new(pointer)
            .map(|pointer| Self { pointer })
            .ok_or(())
    }

    fn to_string(&self) -> Result<String, ()> {
        let mut length = 0;
        while length < MAX_PATH_UNITS {
            // SAFETY: Windows returns a NUL-terminated CoTaskMem UTF-16 path.
            // The bound prevents an unbounded scan if a broken implementation
            // violates that contract.
            let unit = unsafe { *self.pointer.as_ptr().add(length) };
            if unit == 0 {
                // SAFETY: the same Windows allocation is valid through its
                // documented terminating NUL, so this bounded prefix is valid.
                let units = unsafe { slice::from_raw_parts(self.pointer.as_ptr(), length) };
                return decode_path_units(units);
            }
            length += 1;
        }
        Err(())
    }
}

impl Drop for CoTaskMemWide {
    fn drop(&mut self) {
        // SAFETY: this pointer was returned by the documented `CoTaskMem`
        // allocation contract and is released exactly once by this owner.
        unsafe { CoTaskMemFree(self.pointer.as_ptr().cast()) };
    }
}

const fn succeeded(result: Hresult) -> bool {
    result >= S_OK
}

const fn folder_options(current: u32) -> u32 {
    let incompatible = FOS_OVERWRITEPROMPT | FOS_STRICTFILETYPES | FOS_FILEMUSTEXIST;
    (current & !incompatible)
        | FOS_PICKFOLDERS
        | FOS_FORCEFILESYSTEM
        | FOS_PATHMUSTEXIST
        | FOS_DONTADDTORECENT
}

fn decode_path_units(units: &[u16]) -> Result<String, ()> {
    if units.is_empty() {
        return Err(());
    }
    String::from_utf16(units).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{
        CLSID_FILE_OPEN_DIALOG, FOS_DONTADDTORECENT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
        FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FOS_STRICTFILETYPES, Guid,
        IID_I_FILE_OPEN_DIALOG, decode_path_units, folder_options,
    };

    #[test]
    fn folder_options_keep_only_compatible_defaults() {
        let configured = folder_options(
            FOS_OVERWRITEPROMPT | FOS_STRICTFILETYPES | FOS_FILEMUSTEXIST | 0x0000_0001,
        );
        assert_eq!(configured & FOS_OVERWRITEPROMPT, 0);
        assert_eq!(configured & FOS_STRICTFILETYPES, 0);
        assert_eq!(configured & FOS_FILEMUSTEXIST, 0);
        assert_ne!(configured & FOS_PICKFOLDERS, 0);
        assert_ne!(configured & FOS_FORCEFILESYSTEM, 0);
        assert_ne!(configured & FOS_PATHMUSTEXIST, 0);
        assert_ne!(configured & FOS_DONTADDTORECENT, 0);
        assert_ne!(configured & 0x0000_0001, 0);
    }

    #[test]
    fn folder_guids_match_the_windows_sdk_contract() {
        assert_eq!(
            CLSID_FILE_OPEN_DIALOG,
            Guid::new(
                0xdc1c_5a9c,
                0xe88a,
                0x4dde,
                [0xa5, 0xa1, 0x60, 0xf8, 0x2a, 0x20, 0xae, 0xf7],
            )
        );
        assert_eq!(
            IID_I_FILE_OPEN_DIALOG,
            Guid::new(
                0xd57c_7288,
                0xd4ad,
                0x4768,
                [0xbe, 0x02, 0x9d, 0x96, 0x95, 0x32, 0xd9, 0x60],
            )
        );
    }

    #[test]
    fn path_decoder_rejects_empty_and_invalid_utf16() {
        assert_eq!(decode_path_units(&[]), Err(()));
        assert_eq!(decode_path_units(&[0xd800]), Err(()));
        assert_eq!(
            decode_path_units(&r"C:\\Users\\Owner".encode_utf16().collect::<Vec<_>>()),
            Ok(r"C:\\Users\\Owner".to_owned())
        );
    }
}
