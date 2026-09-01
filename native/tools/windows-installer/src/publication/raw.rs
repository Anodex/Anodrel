//! Narrow direct Advapi32 write for the fixed Anodrel machine policy value.

use std::ptr;

use crate::{PublicationError, UpdatePublicationError};

type HKey = isize;

const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
const KEY_QUERY_VALUE: u32 = 0x0001;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_WOW64_64KEY: u32 = 0x0100;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_MORE_DATA: i32 = 234;
const POLICY_PREFIX: &str = "Software\\Anodrel\\Applications\\";
const RECORD_VALUE_NAME: &str = "record";
const PREVIOUS_VALUE_NAME: &str = "previous";
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
    fn RegOpenKeyExW(
        key: HKey,
        sub_key: *const u16,
        options: u32,
        access: u32,
        result: *mut HKey,
    ) -> i32;
    fn RegQueryValueExW(
        key: HKey,
        value_name: *const u16,
        reserved: *mut u32,
        value_type: *mut u32,
        data: *mut u8,
        data_size: *mut u32,
    ) -> i32;
    fn RegCloseKey(key: HKey) -> i32;
}

/// Writes the one fixed registry value for an already validated release.
pub(super) fn write_record(application_id: &str, record: &str) -> Result<(), PublicationError> {
    let data = encode_record(record)?;
    let path = wide_null(&policy_path(application_id));
    let key = create_key(&path)?;
    write_value(&key, RECORD_VALUE_NAME, &data).map_err(status_error)
}

/// Retains the fixed selected record before writing the fixed update record.
pub(super) fn retain_current_then_write_update(
    application_id: &str,
    record: &str,
) -> Result<(), UpdatePublicationError> {
    let data = encode_update_record(record)?;
    let path = wide_null(&policy_path(application_id));
    let key = open_existing_update_key(&path)?;
    let current = read_existing_record(&key)?;
    write_value(&key, PREVIOUS_VALUE_NAME, &current).map_err(update_status_error)?;
    write_value(&key, RECORD_VALUE_NAME, &data).map_err(update_status_error)
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

/// Returns the one private retained update-policy value name.
#[cfg(test)]
pub(super) const fn previous_value_name() -> &'static str {
    PREVIOUS_VALUE_NAME
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

fn encode_update_record(record: &str) -> Result<Vec<u16>, UpdatePublicationError> {
    if record.contains('\0') {
        return Err(UpdatePublicationError::NewRecordEncodingInvalid);
    }
    let encoded = wide_null(record);
    (encoded.len() * 2 <= MAX_RECORD_UTF16_BYTES)
        .then_some(encoded)
        .ok_or(UpdatePublicationError::NewRecordEncodingInvalid)
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

fn open_existing_update_key(path: &[u16]) -> Result<RegistryKey, UpdatePublicationError> {
    let mut key = 0_isize;
    // SAFETY: path is NUL terminated and `key` is one output slot. This update
    // path reads and writes only the fixed values in the 64-bit machine view.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    (status == ERROR_SUCCESS)
        .then_some(RegistryKey(key))
        .ok_or(update_status_error(status))
}

fn read_existing_record(key: &RegistryKey) -> Result<Vec<u16>, UpdatePublicationError> {
    let value = wide_null(RECORD_VALUE_NAME);
    let mut value_type = 0_u32;
    let mut byte_length = 0_u32;
    // SAFETY: `key` is live, value is NUL terminated, and Windows writes only
    // the two declared u32 output values during this size query.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            ptr::null_mut(),
            &mut byte_length,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(update_status_error(status));
    }
    if value_type != REG_SZ || byte_length == 0 || !byte_length.is_multiple_of(2) {
        return Err(UpdatePublicationError::ExistingRecordMalformed);
    }
    if byte_length as usize > MAX_RECORD_UTF16_BYTES {
        return Err(UpdatePublicationError::ExistingRecordMalformed);
    }
    let initial_length = byte_length;
    let mut data = vec![0_u16; (byte_length / 2) as usize];
    // SAFETY: data has exactly the UTF-16 capacity returned by the first query.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            data.as_mut_ptr().cast(),
            &mut byte_length,
        )
    };
    if status == ERROR_MORE_DATA || byte_length != initial_length || value_type != REG_SZ {
        return Err(UpdatePublicationError::ExistingRecordChanged);
    }
    if status != ERROR_SUCCESS {
        return Err(update_status_error(status));
    }
    valid_record_utf16(&data)
        .then_some(data)
        .ok_or(UpdatePublicationError::ExistingRecordMalformed)
}

fn valid_record_utf16(data: &[u16]) -> bool {
    let Some((&0, body)) = data.split_last() else {
        return false;
    };
    !body.contains(&0) && String::from_utf16(body).is_ok()
}

fn write_value(key: &RegistryKey, name: &str, data: &[u16]) -> Result<(), i32> {
    let value = wide_null(name);
    let byte_length = u32::try_from(data.len() * 2).map_err(|_| ERROR_MORE_DATA)?;
    // SAFETY: key owns a live set-value registry handle; name and data are
    // bounded NUL-terminated UTF-16 buffers with the exact REG_SZ byte length.
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
    (status == ERROR_SUCCESS).then_some(()).ok_or(status)
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

fn update_status_error(status: i32) -> UpdatePublicationError {
    match status {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => {
            UpdatePublicationError::ExistingRecordUnavailable
        }
        ERROR_ACCESS_DENIED => UpdatePublicationError::AccessDenied,
        ERROR_MORE_DATA => UpdatePublicationError::ExistingRecordChanged,
        _ => UpdatePublicationError::RegistryUnavailable,
    }
}

struct RegistryKey(HKey);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: This guard owns the one successful Registry API handle.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}
