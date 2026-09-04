//! Direct registry persistence for one fixed Apps & features entry.

use std::path::Path;

use super::VerifiedAppsFeaturesTarget;

type HKey = isize;

const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_CREATE_SUB_KEY: u32 = 0x0004;
const KEY_WOW64_64KEY: u32 = 0x0100;
const DELETE: u32 = 0x0001_0000;
const REG_SZ: u32 = 1;
const REG_DWORD: u32 = 4;
const ERROR_SUCCESS: i32 = 0;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const APPS_FEATURES_PREFIX: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Anodrel\\";
const DISPLAY_NAME: &str = "DisplayName";
const DISPLAY_VERSION: &str = "DisplayVersion";
const NO_MODIFY: &str = "NoModify";
const NO_REPAIR: &str = "NoRepair";
const PUBLISHER: &str = "Publisher";
const UNINSTALL_STRING: &str = "UninstallString";

#[link(name = "Advapi32")]
unsafe extern "system" {
    fn RegCreateKeyExW(
        key: HKey,
        sub_key: *const u16,
        reserved: u32,
        class: *const u16,
        options: u32,
        access: u32,
        security_attributes: *const core::ffi::c_void,
        result: *mut HKey,
        disposition: *mut u32,
    ) -> i32;
    fn RegOpenKeyExW(
        key: HKey,
        sub_key: *const u16,
        options: u32,
        access: u32,
        result: *mut HKey,
    ) -> i32;
    fn RegSetValueExW(
        key: HKey,
        value_name: *const u16,
        reserved: u32,
        value_type: u32,
        data: *const u8,
        data_size: u32,
    ) -> i32;
    fn RegDeleteValueW(key: HKey, value_name: *const u16) -> i32;
    fn RegDeleteKeyExW(key: HKey, sub_key: *const u16, access: u32, reserved: u32) -> i32;
    fn RegCloseKey(key: HKey) -> i32;
}

pub(super) fn write(target: &VerifiedAppsFeaturesTarget) -> Result<(), ()> {
    let key = create_key(&entry_path(&target.application_id))?;
    write_string(&key, DISPLAY_NAME, &target.display_name)?;
    write_string(&key, PUBLISHER, &target.publisher_name)?;
    write_string(&key, DISPLAY_VERSION, &display_version(target.version))?;
    write_string(
        &key,
        UNINSTALL_STRING,
        &uninstall_command(&target.uninstaller_path),
    )?;
    write_dword(&key, NO_MODIFY, 1)?;
    write_dword(&key, NO_REPAIR, 1)
}

pub(super) fn remove(target: &VerifiedAppsFeaturesTarget) -> Result<(), ()> {
    let parent = APPS_FEATURES_PREFIX.trim_end_matches('\\');
    let key = match open_key(parent, KEY_SET_VALUE | KEY_CREATE_SUB_KEY | DELETE) {
        Ok(key) => key,
        Err(OpenKeyError::Missing) => return Ok(()),
        Err(OpenKeyError::Failed) => return Err(()),
    };
    let name = wide(&target.application_id)?;
    let entry = match open_key(&entry_path(&target.application_id), KEY_SET_VALUE) {
        Ok(entry) => entry,
        Err(OpenKeyError::Missing) => return Ok(()),
        Err(OpenKeyError::Failed) => return Err(()),
    };
    for value in [
        DISPLAY_NAME,
        PUBLISHER,
        DISPLAY_VERSION,
        UNINSTALL_STRING,
        NO_MODIFY,
        NO_REPAIR,
    ] {
        delete_value(&entry, value)?;
    }
    let status = unsafe { RegDeleteKeyExW(key.0, name.as_ptr(), KEY_WOW64_64KEY, 0) };
    (status == ERROR_SUCCESS).then_some(()).ok_or(())
}

fn create_key(path: &str) -> Result<RegistryKey, ()> {
    let path = wide(path)?;
    let mut key = 0_isize;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE | KEY_WOW64_64KEY,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    (status == ERROR_SUCCESS)
        .then_some(RegistryKey(key))
        .ok_or(())
}

fn open_key(path: &str, access: u32) -> Result<RegistryKey, OpenKeyError> {
    let path = wide(path).map_err(|_| OpenKeyError::Failed)?;
    let mut key = 0_isize;
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            access | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(RegistryKey(key))
    } else if status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND {
        Err(OpenKeyError::Missing)
    } else {
        Err(OpenKeyError::Failed)
    }
}

fn write_string(key: &RegistryKey, name: &str, value: &str) -> Result<(), ()> {
    let name = wide(name)?;
    let value = wide(value)?;
    write_value(
        key,
        name.as_ptr(),
        REG_SZ,
        value.as_ptr().cast(),
        value.len() * 2,
    )
}

fn write_dword(key: &RegistryKey, name: &str, value: u32) -> Result<(), ()> {
    let name = wide(name)?;
    write_value(
        key,
        name.as_ptr(),
        REG_DWORD,
        (&value as *const u32).cast(),
        std::mem::size_of::<u32>(),
    )
}

fn write_value(
    key: &RegistryKey,
    name: *const u16,
    kind: u32,
    data: *const u8,
    byte_length: usize,
) -> Result<(), ()> {
    let byte_length = u32::try_from(byte_length).map_err(|_| ())?;
    let status = unsafe { RegSetValueExW(key.0, name, 0, kind, data, byte_length) };
    (status == ERROR_SUCCESS).then_some(()).ok_or(())
}

fn delete_value(key: &RegistryKey, name: &str) -> Result<(), ()> {
    let name = wide(name)?;
    let status = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
    (status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND)
        .then_some(())
        .ok_or(())
}

fn entry_path(application_id: &str) -> String {
    format!("{APPS_FEATURES_PREFIX}{application_id}")
}

fn display_version(version: crate::PackageVersion) -> String {
    format!(
        "{}.{}.{}",
        version.major(),
        version.minor(),
        version.patch()
    )
}

fn uninstall_command(path: &Path) -> String {
    format!("\"{}\" remove", path.display())
}

fn wide(value: &str) -> Result<Vec<u16>, ()> {
    (!value.contains('\0')).then_some(()).ok_or(())?;
    Ok(value.encode_utf16().chain(Some(0)).collect())
}

struct RegistryKey(HKey);

enum OpenKeyError {
    Missing,
    Failed,
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{display_version, entry_path, uninstall_command};

    #[test]
    fn registry_location_and_command_are_fixed_from_private_policy_data() {
        assert_eq!(
            entry_path("org.anodrel.sample"),
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Anodrel\\org.anodrel.sample"
        );
        assert_eq!(
            display_version(crate::PackageVersion::new(1, 2, 3)),
            "1.2.3"
        );
        assert_eq!(
            uninstall_command(std::path::Path::new(
                "C:\\Program Files\\Anodrel\\remove.exe"
            )),
            "\"C:\\Program Files\\Anodrel\\remove.exe\" remove"
        );
    }
}
