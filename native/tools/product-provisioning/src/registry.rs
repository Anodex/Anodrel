//! The single machine-policy value this development helper may write.
//!
//! The native host reads machine policy and never writes it. This module is the
//! only writer in the repository, it lives in a development tool the host does
//! not link, and it is deliberately unable to name a hive, key path, or value
//! name: all three are compile-time constants matching `docs/LAUNCH.md`. The
//! application ID is validated by the caller before it reaches this module.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

use std::{fmt, ptr};

type HKey = isize;

const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_WOW64_64KEY: u32 = 0x0100;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const ERROR_ACCESS_DENIED: i32 = 5;
const POLICY_KEY_PREFIX: &str = "Software\\Anodrel\\Applications\\";
const RECORD_VALUE_NAME: &str = "record";

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
    fn RegSetValueExW(
        key: HKey,
        value_name: *const u16,
        reserved: u32,
        value_type: u32,
        data: *const u8,
        data_size: u32,
    ) -> i32;
    fn RegDeleteKeyExW(key: HKey, sub_key: *const u16, access: u32, reserved: u32) -> i32;
    fn RegCloseKey(key: HKey) -> i32;
}

/// Writes the one `record` value for a prevalidated application ID.
///
/// The caller must have composed and validated the record first. This performs
/// no validation of its own beyond size and encoding, because a record that
/// reached here has already passed the host's own parser.
pub fn write_record(application_id: &str, record: &str) -> Result<(), RegistryError> {
    let data = utf16_with_nul(record)?;
    let key = create_policy_key(application_id)?;
    let value_name = wide_null(RECORD_VALUE_NAME);
    let byte_length = u32::try_from(data.len() * 2).map_err(|_| RegistryError::RecordTooLarge)?;

    // SAFETY: `key` is a live set-value registry key, `value_name` is NUL
    // terminated, and `data` is exactly `byte_length` readable bytes of UTF-16
    // ending in the terminator REG_SZ requires.
    let status = unsafe {
        RegSetValueExW(
            key.0,
            value_name.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr().cast(),
            byte_length,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(status_error(status))
    }
}

/// Removes the fixture's policy key, including its one record value.
///
/// A missing key is success: removal must be safe to run twice.
pub fn remove_record(application_id: &str) -> Result<(), RegistryError> {
    let path = wide_null(&format!("{POLICY_KEY_PREFIX}{application_id}"));
    // SAFETY: `path` is a NUL-terminated machine-relative key path and the
    // 64-bit view flag matches the one the host reads from.
    let status = unsafe { RegDeleteKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), KEY_WOW64_64KEY, 0) };
    match status {
        ERROR_SUCCESS | ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(()),
        status => Err(status_error(status)),
    }
}

fn create_policy_key(application_id: &str) -> Result<RegistryKey, RegistryError> {
    let path = wide_null(&format!("{POLICY_KEY_PREFIX}{application_id}"));
    let mut key = 0_isize;
    // SAFETY: `path` is NUL terminated and `key` points to one initialized HKEY
    // slot. The requested access is set-value only, in the 64-bit view.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            ptr::null(),
            0,
            KEY_SET_VALUE | KEY_WOW64_64KEY,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };
    if status == ERROR_SUCCESS {
        Ok(RegistryKey(key))
    } else {
        Err(status_error(status))
    }
}

/// Converts a record to the exact `REG_SZ` representation the reader requires:
/// valid UTF-16 with one terminating NUL and no embedded NUL.
fn utf16_with_nul(record: &str) -> Result<Vec<u16>, RegistryError> {
    if record.contains('\0') {
        return Err(RegistryError::RecordMalformed);
    }
    let data = wide_null(record);
    if data.len() * 2 > 32 * 1024 {
        return Err(RegistryError::RecordTooLarge);
    }
    Ok(data)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn status_error(status: i32) -> RegistryError {
    match status {
        ERROR_ACCESS_DENIED => RegistryError::AccessDenied,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => RegistryError::KeyNotFound,
        _ => RegistryError::RegistryUnavailable,
    }
}

struct RegistryKey(HKey);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: RegistryKey exists only after RegCreateKeyExW succeeded, and
        // dropping it closes this one registry handle exactly once.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

/// A safe failure category while changing machine policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryError {
    AccessDenied,
    KeyNotFound,
    RecordTooLarge,
    RecordMalformed,
    RegistryUnavailable,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::AccessDenied => {
                "machine policy cannot be changed; run this from an elevated shell"
            }
            Self::KeyNotFound => "the fixture machine-policy key does not exist",
            Self::RecordTooLarge => "the fixture record exceeds the registry value limit",
            Self::RecordMalformed => "the fixture record cannot be stored as a registry string",
            Self::RegistryUnavailable => "Windows machine policy is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::{POLICY_KEY_PREFIX, RECORD_VALUE_NAME, RegistryError, utf16_with_nul, wide_null};

    #[test]
    fn the_written_location_matches_the_location_the_host_reads() {
        // `anodrel-windows-policy` opens exactly this path and value name.
        assert_eq!(POLICY_KEY_PREFIX, "Software\\Anodrel\\Applications\\");
        assert_eq!(RECORD_VALUE_NAME, "record");
    }

    #[test]
    fn a_record_is_stored_with_one_terminating_nul_and_no_embedded_nul() {
        let data = utf16_with_nul("{}").expect("the record encodes");
        assert_eq!(data, vec![b'{' as u16, b'}' as u16, 0]);
        assert_eq!(utf16_with_nul("{\0}"), Err(RegistryError::RecordMalformed));
    }

    #[test]
    fn an_oversized_record_is_refused_before_any_registry_call() {
        assert_eq!(
            utf16_with_nul(&"x".repeat(16 * 1024 + 1)),
            Err(RegistryError::RecordTooLarge)
        );
    }

    #[test]
    fn key_paths_are_machine_relative_and_terminated() {
        let path = wide_null(&format!("{POLICY_KEY_PREFIX}org.anodrel.product-fixture"));
        assert_eq!(path.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&path[..path.len() - 1]).expect("the path is valid UTF-16"),
            "Software\\Anodrel\\Applications\\org.anodrel.product-fixture"
        );
    }
}
