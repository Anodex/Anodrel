//! Narrow direct Advapi32 write for the fixed Anodrel machine policy value.

use std::ptr;

use crate::PublicationError;

type HKey = isize;

const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_WOW64_64KEY: u32 = 0x0100;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;
const ERROR_ACCESS_DENIED: i32 = 5;
const POLICY_PREFIX: &str = "Software\\Anodrel\\Applications\\";
const RECORD_VALUE_NAME: &str = "record";
const MAX_RECORD_UTF16_BYTES: usize = 32 * 1024;

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
    fn RegCloseKey(key: HKey) -> i32;
}

/// Writes the one fixed registry value for an already validated release.
pub(super) fn write_record(application_id: &str, record: &str) -> Result<(), PublicationError> {
    let data = encode_record(record)?;
    let path = wide_null(&policy_path(application_id));
    let key = create_key(&path)?;
    let value = wide_null(RECORD_VALUE_NAME);
    let byte_length =
        u32::try_from(data.len() * 2).map_err(|_| PublicationError::RecordEncodingInvalid)?;
    // SAFETY: The handle is live and set-value only; name and data are valid
    // NUL-terminated UTF-16 storage with the exact byte count for `REG_SZ`.
    let status = unsafe {
        RegSetValueExW(
            key.0,
            value.as_ptr(),
            0,
            REG_SZ,
            data.as_ptr().cast(),
            byte_length,
        )
    };
    (status == ERROR_SUCCESS)
        .then_some(())
        .ok_or(status_error(status))
}

/// Renders the sole host-read machine policy key for a validated identity.
pub(super) fn policy_path(application_id: &str) -> String {
    format!("{POLICY_PREFIX}{application_id}")
}

/// Returns the one fixed policy value name.
#[cfg(test)]
pub(super) const fn record_value_name() -> &'static str {
    RECORD_VALUE_NAME
}

/// Encodes exactly one `REG_SZ` string with no embedded terminator.
pub(super) fn encode_record(record: &str) -> Result<Vec<u16>, PublicationError> {
    if record.contains('\0') {
        return Err(PublicationError::RecordEncodingInvalid);
    }
    let encoded = wide_null(record);
    (encoded.len() * 2 <= MAX_RECORD_UTF16_BYTES)
        .then_some(encoded)
        .ok_or(PublicationError::RecordEncodingInvalid)
}

fn create_key(path: &[u16]) -> Result<RegistryKey, PublicationError> {
    let mut key = 0_isize;
    // SAFETY: The path is NUL-terminated; result points to one HKEY slot. The
    // 64-bit machine view and set-value-only access match the host reader.
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
    (status == ERROR_SUCCESS)
        .then_some(RegistryKey(key))
        .ok_or(status_error(status))
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn status_error(status: i32) -> PublicationError {
    match status {
        ERROR_ACCESS_DENIED => PublicationError::AccessDenied,
        _ => PublicationError::RegistryUnavailable,
    }
}

struct RegistryKey(HKey);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: This guard owns the one successful Registry API handle.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}
