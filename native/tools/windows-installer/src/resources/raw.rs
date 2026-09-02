//! Narrow Kernel32 resource access for current and locked external images.

use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

use super::EmbeddedReleaseError;

type ModuleHandle = *mut c_void;
type ResourceHandle = *mut c_void;
type ResourceDataHandle = *mut c_void;

const LOAD_LIBRARY_AS_IMAGE_RESOURCE: u32 = 0x0000_0020;
const LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE: u32 = 0x0000_0040;
const RT_RCDATA: u16 = 10;

unsafe extern "system" {
    fn FreeLibrary(module: ModuleHandle) -> i32;
    fn GetModuleHandleW(module_name: *const u16) -> ModuleHandle;
    fn LoadLibraryExW(file_name: *const u16, file: *mut c_void, flags: u32) -> ModuleHandle;
    fn FindResourceW(
        module: ModuleHandle,
        name: *const u16,
        resource_type: *const u16,
    ) -> ResourceHandle;
    fn SizeofResource(module: ModuleHandle, resource: ResourceHandle) -> u32;
    fn LoadResource(module: ModuleHandle, resource: ResourceHandle) -> ResourceDataHandle;
    fn LockResource(resource_data: ResourceDataHandle) -> *const c_void;
}

/// One non-executing resource mapping that prevents writes to its image.
pub(super) struct LockedResourceImage {
    module: ModuleHandle,
}

impl LockedResourceImage {
    /// Maps one absolute installer image only for fixed resource access.
    pub(super) fn open(path: &Path) -> Result<Self, EmbeddedReleaseError> {
        if !path.is_absolute() {
            return Err(EmbeddedReleaseError::ImageUnavailable);
        }
        let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
        wide.push(0);
        // SAFETY: `wide` is a null-terminated absolute Windows path. The two
        // resource flags load no code and retain exclusive write protection for
        // this mapping until the matching FreeLibrary call in Drop.
        let module = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_AS_IMAGE_RESOURCE | LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE,
            )
        };
        if module.is_null() {
            return Err(EmbeddedReleaseError::ImageUnavailable);
        }
        Ok(Self { module })
    }

    /// Returns one fixed resource while the external mapping remains alive.
    pub(super) fn resource(&self, identifier: u16) -> Result<&[u8], EmbeddedReleaseError> {
        resource(self.module, identifier)
    }
}

impl Drop for LockedResourceImage {
    fn drop(&mut self) {
        // SAFETY: this module handle was returned by LoadLibraryExW exactly
        // once in `open` and is owned solely by this guard.
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}

/// Returns a fixed resource borrowed for the current executable image lifetime.
pub(super) fn current_resource(identifier: u16) -> Result<&'static [u8], EmbeddedReleaseError> {
    // SAFETY: A null module name requests the current process executable. That
    // image stays loaded for process lifetime; no resource handle is freed.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_null() {
        return Err(EmbeddedReleaseError::CurrentImageUnavailable);
    }
    resource(module, identifier)
}

fn resource<'image>(
    module: ModuleHandle,
    identifier: u16,
) -> Result<&'image [u8], EmbeddedReleaseError> {
    // SAFETY: Integer resource identifiers are represented by their low-word
    // pointer value. Both values are fixed private constants, not caller input.
    let resource = unsafe {
        FindResourceW(
            module,
            make_integer_resource(identifier),
            make_integer_resource(RT_RCDATA),
        )
    };
    if resource.is_null() {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: `resource` was returned for `module` by FindResourceW above.
    let length = unsafe { SizeofResource(module, resource) };
    if length == 0 {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: `resource` remains associated with the loaded image mapping.
    let resource_data = unsafe { LoadResource(module, resource) };
    if resource_data.is_null() {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: LockResource returns a readable pointer to exactly SizeofResource
    // bytes that remains valid while the corresponding image is mapped.
    let bytes = unsafe { LockResource(resource_data).cast::<u8>() };
    if bytes.is_null() {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: The pointer and nonzero length come from the same resource. The
    // caller keeps either the current image or a LockedResourceImage alive.
    Ok(unsafe { std::slice::from_raw_parts(bytes, length as usize) })
}

const fn make_integer_resource(identifier: u16) -> *const u16 {
    identifier as usize as *const u16
}
