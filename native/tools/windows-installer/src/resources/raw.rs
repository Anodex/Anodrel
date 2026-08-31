//! Narrow Kernel32 resource access for the current executable image.

use std::ffi::c_void;

use super::EmbeddedReleaseError;

type ModuleHandle = *mut c_void;
type ResourceHandle = *mut c_void;
type ResourceDataHandle = *mut c_void;

const RT_RCDATA: u16 = 10;

unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> ModuleHandle;
    fn FindResourceW(
        module: ModuleHandle,
        name: *const u16,
        resource_type: *const u16,
    ) -> ResourceHandle;
    fn SizeofResource(module: ModuleHandle, resource: ResourceHandle) -> u32;
    fn LoadResource(module: ModuleHandle, resource: ResourceHandle) -> ResourceDataHandle;
    fn LockResource(resource_data: ResourceDataHandle) -> *const c_void;
}

/// Returns a fixed resource borrowed for the current executable image lifetime.
pub(super) fn current_resource(identifier: u16) -> Result<&'static [u8], EmbeddedReleaseError> {
    // SAFETY: A null module name requests the current process executable. That
    // image stays loaded for process lifetime; no resource handle is freed.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_null() {
        return Err(EmbeddedReleaseError::CurrentImageUnavailable);
    }
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
    // SAFETY: `resource` remains associated with the current loaded image.
    let resource_data = unsafe { LoadResource(module, resource) };
    if resource_data.is_null() {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: LockResource returns a readable pointer to exactly SizeofResource
    // bytes that remains valid while the current executable image is loaded.
    let bytes = unsafe { LockResource(resource_data).cast::<u8>() };
    if bytes.is_null() {
        return Err(EmbeddedReleaseError::ResourceUnavailable);
    }
    // SAFETY: The pointer and nonzero length come from the same current-image
    // resource. The current executable cannot unload before process exit.
    Ok(unsafe { std::slice::from_raw_parts(bytes, length as usize) })
}

const fn make_integer_resource(identifier: u16) -> *const u16 {
    identifier as usize as *const u16
}
