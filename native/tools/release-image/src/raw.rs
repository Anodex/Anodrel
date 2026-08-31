//! Direct Kernel32 PE resource update and verification calls.

use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

use anodrel_windows_installer::{RELEASE_MANIFEST_RESOURCE_ID, RELEASE_PAYLOAD_RESOURCE_ID};

use crate::ReleaseImageError;

type Handle = *mut c_void;
type ModuleHandle = *mut c_void;
type ResourceHandle = *mut c_void;
type ResourceDataHandle = *mut c_void;

const RT_RCDATA: u16 = 10;
const LOAD_LIBRARY_AS_DATAFILE: u32 = 0x0000_0002;

unsafe extern "system" {
    fn BeginUpdateResourceW(file_name: *const u16, delete_existing: i32) -> Handle;
    fn UpdateResourceW(
        update: Handle,
        resource_type: *const u16,
        name: *const u16,
        language: u16,
        data: *const c_void,
        byte_count: u32,
    ) -> i32;
    fn EndUpdateResourceW(update: Handle, discard: i32) -> i32;
    fn LoadLibraryExW(file_name: *const u16, file: Handle, flags: u32) -> ModuleHandle;
    fn FreeLibrary(module: ModuleHandle) -> i32;
    fn FindResourceW(
        module: ModuleHandle,
        name: *const u16,
        resource_type: *const u16,
    ) -> ResourceHandle;
    fn SizeofResource(module: ModuleHandle, resource: ResourceHandle) -> u32;
    fn LoadResource(module: ModuleHandle, resource: ResourceHandle) -> ResourceDataHandle;
    fn LockResource(resource_data: ResourceDataHandle) -> *const c_void;
}

/// Writes both fixed release resources as one transaction.
pub(super) fn write_resources(
    output: &Path,
    manifest: &[u8],
    payload: &[u8],
) -> Result<(), ReleaseImageError> {
    let output = wide_path(output);
    // SAFETY: The caller validated an absolute, newly copied output path. This
    // requests an update handle for that non-running output only.
    let handle = unsafe { BeginUpdateResourceW(output.as_ptr(), 0) };
    if handle.is_null() {
        return Err(ReleaseImageError::ResourceTransactionUnavailable);
    }
    let transaction = ResourceTransaction { handle };
    transaction.write(RELEASE_MANIFEST_RESOURCE_ID, manifest)?;
    transaction.write(RELEASE_PAYLOAD_RESOURCE_ID, payload)?;
    transaction.commit()
}

/// Requires both fixed output resources to exactly equal their source bytes.
pub(super) fn resources_match(
    output: &Path,
    manifest: &[u8],
    payload: &[u8],
) -> Result<(), ReleaseImageError> {
    let output = wide_path(output);
    // SAFETY: The path names a just-created output PE. The data-file flag avoids
    // running code or resolving imports while resources are inspected.
    let module = unsafe {
        LoadLibraryExW(
            output.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_AS_DATAFILE,
        )
    };
    if module.is_null() {
        return Err(ReleaseImageError::ResourceVerificationFailed);
    }
    let image = LoadedDataImage { module };
    let manifest_matches = image.resource_matches(RELEASE_MANIFEST_RESOURCE_ID, manifest);
    let payload_matches = image.resource_matches(RELEASE_PAYLOAD_RESOURCE_ID, payload);
    (manifest_matches && payload_matches)
        .then_some(())
        .ok_or(ReleaseImageError::ResourceVerificationFailed)
}

struct ResourceTransaction {
    handle: Handle,
}

impl ResourceTransaction {
    fn write(&self, identifier: u16, data: &[u8]) -> Result<(), ReleaseImageError> {
        let byte_count =
            u32::try_from(data.len()).map_err(|_| ReleaseImageError::ResourceWriteFailed)?;
        // SAFETY: The transaction handle comes from BeginUpdateResourceW. The
        // identifiers are fixed private RCDATA values, and `data` stays alive
        // through this synchronous call with the exact supplied byte count.
        let updated = unsafe {
            UpdateResourceW(
                self.handle,
                make_integer_resource(RT_RCDATA),
                make_integer_resource(identifier),
                0,
                data.as_ptr().cast::<c_void>(),
                byte_count,
            )
        };
        (updated != 0)
            .then_some(())
            .ok_or(ReleaseImageError::ResourceWriteFailed)
    }

    fn commit(mut self) -> Result<(), ReleaseImageError> {
        // SAFETY: EndUpdateResourceW consumes this transaction and commits its
        // accumulated updates as one operation when discard is false.
        let committed = unsafe { EndUpdateResourceW(self.handle, 0) };
        self.handle = std::ptr::null_mut();
        (committed != 0)
            .then_some(())
            .ok_or(ReleaseImageError::ResourceCommitFailed)
    }
}

impl Drop for ResourceTransaction {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: A still-owned transaction has not been committed. Windows
            // discards all accumulated updates when the discard flag is true.
            let _ = unsafe { EndUpdateResourceW(self.handle, 1) };
        }
    }
}

struct LoadedDataImage {
    module: ModuleHandle,
}

impl LoadedDataImage {
    fn resource_matches(&self, identifier: u16, expected: &[u8]) -> bool {
        // SAFETY: This data-only module remains loaded by this guard. Both
        // resource identifiers are fixed private RCDATA values.
        let resource = unsafe {
            FindResourceW(
                self.module,
                make_integer_resource(identifier),
                make_integer_resource(RT_RCDATA),
            )
        };
        if resource.is_null() {
            return false;
        }
        // SAFETY: The resource handle belongs to the guarded loaded module.
        let length = unsafe { SizeofResource(self.module, resource) };
        if length as usize != expected.len() {
            return false;
        }
        // SAFETY: The resource handle belongs to the guarded loaded module.
        let data = unsafe { LoadResource(self.module, resource) };
        if data.is_null() {
            return false;
        }
        // SAFETY: LockResource exposes exactly SizeofResource bytes while the
        // data-only module stays loaded by this guard.
        let bytes = unsafe { LockResource(data).cast::<u8>() };
        if bytes.is_null() {
            return false;
        }
        // SAFETY: Pointer and byte count came from the same locked resource.
        unsafe { std::slice::from_raw_parts(bytes, expected.len()) == expected }
    }
}

impl Drop for LoadedDataImage {
    fn drop(&mut self) {
        // SAFETY: This guard owns the successful LoadLibraryExW handle exactly
        // once and does not expose it outside the module.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

const fn make_integer_resource(identifier: u16) -> *const u16 {
    identifier as usize as *const u16
}
