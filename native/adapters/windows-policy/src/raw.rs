//! Narrow direct Advapi32 bindings for fixed machine policy registry values.

use std::ptr;

use crate::PolicyStoreError;

type HKey = isize;

const HKEY_LOCAL_MACHINE: HKey = 0x8000_0002_usize as HKey;
const KEY_QUERY_VALUE: u32 = 0x0001;
const KEY_WOW64_64KEY: u32 = 0x0100;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_MORE_DATA: i32 = 234;
const MAX_RECORD_UTF16_BYTES: u32 = 32 * 1024;
const POLICY_KEY_PREFIX: &str = "Software\\Anodrel\\Applications\\";
const CURRENT_RECORD_VALUE: &str = "record";
const PREVIOUS_RECORD_VALUE: &str = "previous";

#[link(name = "Advapi32")]
unsafe extern "system" {
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

/// Reads the one allowed registry value for a prevalidated application ID.
pub(super) fn read_record(application_id: &str) -> Result<String, PolicyStoreError> {
    read_value(application_id, PolicyValue::Current)
}

/// Reads the one fixed retained registry value for a prevalidated application ID.
pub(super) fn read_previous_record(application_id: &str) -> Result<String, PolicyStoreError> {
    read_value(application_id, PolicyValue::Previous)
}

fn read_value(application_id: &str, value: PolicyValue) -> Result<String, PolicyStoreError> {
    let key_path = wide_null(&format!("{POLICY_KEY_PREFIX}{application_id}"));
    let key = open_policy_key(&key_path)?;
    let value_name = wide_null(value.name());

    let mut value_type = 0_u32;
    let mut byte_length = 0_u32;
    let status = unsafe {
        // SAFETY: key is a live query-only registry key, value_name is NUL
        // terminated, and Windows writes only the two u32 output values.
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            ptr::null_mut(),
            &mut byte_length,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(status_error(status));
    }
    if value_type != REG_SZ {
        return Err(PolicyStoreError::RecordNotString);
    }
    if byte_length == 0 || !byte_length.is_multiple_of(2) {
        return Err(PolicyStoreError::RecordMalformed);
    }
    if byte_length > MAX_RECORD_UTF16_BYTES {
        return Err(PolicyStoreError::RecordTooLarge);
    }

    let mut data = vec![0_u16; (byte_length / 2) as usize];
    let initial_length = byte_length;
    let status = unsafe {
        // SAFETY: data has space for exactly the byte length returned by the
        // first query and is valid for Windows to fill as raw UTF-16 units.
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            data.as_mut_ptr().cast(),
            &mut byte_length,
        )
    };
    if status == ERROR_MORE_DATA || byte_length != initial_length {
        return Err(PolicyStoreError::RecordChangedDuringRead);
    }
    if status != ERROR_SUCCESS {
        return Err(status_error(status));
    }
    if value_type != REG_SZ {
        return Err(PolicyStoreError::RecordChangedDuringRead);
    }
    decode_record(&data)
}

#[derive(Clone, Copy)]
enum PolicyValue {
    Current,
    Previous,
}

impl PolicyValue {
    const fn name(self) -> &'static str {
        match self {
            Self::Current => CURRENT_RECORD_VALUE,
            Self::Previous => PREVIOUS_RECORD_VALUE,
        }
    }
}

fn open_policy_key(path: &[u16]) -> Result<RegistryKey, PolicyStoreError> {
    let mut key = 0_isize;
    let status = unsafe {
        // SAFETY: path is NUL terminated and result points to one initialized
        // HKEY slot. The requested access is query-only in the 64-bit view.
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(RegistryKey(key))
    } else {
        Err(status_error(status))
    }
}

fn decode_record(data: &[u16]) -> Result<String, PolicyStoreError> {
    let Some((&0, body)) = data.split_last() else {
        return Err(PolicyStoreError::RecordMalformed);
    };
    if body.contains(&0) {
        return Err(PolicyStoreError::RecordMalformed);
    }
    String::from_utf16(body).map_err(|_| PolicyStoreError::RecordMalformed)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn status_error(status: i32) -> PolicyStoreError {
    match status {
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => PolicyStoreError::RecordNotFound,
        ERROR_ACCESS_DENIED => PolicyStoreError::AccessDenied,
        ERROR_MORE_DATA => PolicyStoreError::RecordChangedDuringRead,
        _ => PolicyStoreError::RegistryUnavailable,
    }
}

struct RegistryKey(HKey);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: RegistryKey is constructed only after RegOpenKeyExW
            // succeeds, and dropping it closes this one registry handle.
            let _ = RegCloseKey(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{POLICY_KEY_PREFIX, PolicyValue, decode_record, status_error, wide_null};
    use crate::PolicyStoreError;

    #[test]
    fn registry_key_path_is_machine_relative_and_nul_terminated() {
        let path = wide_null(&format!("{POLICY_KEY_PREFIX}org.anodrel.sample"));
        assert_eq!(path.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&path[..path.len() - 1]).expect("path is valid UTF-16"),
            "Software\\Anodrel\\Applications\\org.anodrel.sample"
        );
    }

    #[test]
    fn policy_value_selection_is_fixed_and_not_caller_supplied() {
        assert_eq!(PolicyValue::Current.name(), "record");
        assert_eq!(PolicyValue::Previous.name(), "previous");
    }

    #[test]
    fn record_decoder_requires_one_terminal_nul_and_valid_utf16() {
        assert!(matches!(
            decode_record(&[b'{' as u16, b'}' as u16, 0]),
            Ok(value) if value == "{}"
        ));
        assert!(matches!(
            decode_record(&[b'{' as u16, 0, b'}' as u16, 0]),
            Err(PolicyStoreError::RecordMalformed)
        ));
        assert!(matches!(
            decode_record(&[0xD800, 0]),
            Err(PolicyStoreError::RecordMalformed)
        ));
    }

    #[test]
    fn registry_statuses_map_to_safe_categories() {
        assert!(matches!(status_error(2), PolicyStoreError::RecordNotFound));
        assert!(matches!(status_error(5), PolicyStoreError::AccessDenied));
        assert!(matches!(
            status_error(234),
            PolicyStoreError::RecordChangedDuringRead
        ));
    }
}
