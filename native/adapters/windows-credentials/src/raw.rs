//! Narrow direct Advapi32 bindings for generic Windows credentials.

use std::{io, os::windows::ffi::OsStrExt, path::Path, ptr, slice};

type Bool = i32;
type Dword = u32;

const CRED_TYPE_GENERIC: Dword = 1;
const CRED_PERSIST_LOCAL_MACHINE: Dword = 2;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_NOT_FOUND: i32 = 1168;
const MAX_WINDOWS_BLOB_BYTES: usize = 5 * 512;
const MAX_ANODREL_SECRET_BYTES: usize = 2 * 1024;

#[repr(C)]
#[derive(Default)]
struct FileTime {
    low_date_time: Dword,
    high_date_time: Dword,
}

#[repr(C)]
struct CredentialW {
    flags: Dword,
    credential_type: Dword,
    target_name: *mut u16,
    comment: *mut u16,
    last_written: FileTime,
    credential_blob_size: Dword,
    credential_blob: *mut u8,
    persist: Dword,
    attribute_count: Dword,
    attributes: *mut core::ffi::c_void,
    target_alias: *mut u16,
    user_name: *mut u16,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn CredWriteW(credential: *const CredentialW, flags: Dword) -> Bool;
    fn CredReadW(
        target_name: *const u16,
        credential_type: Dword,
        flags: Dword,
        credential: *mut *mut CredentialW,
    ) -> Bool;
    fn CredDeleteW(target_name: *const u16, credential_type: Dword, flags: Dword) -> Bool;
    fn CredFree(buffer: *const core::ffi::c_void);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CredentialManagerError {
    NotFound,
    AccessDenied,
    Unavailable,
    StoredSecretInvalid,
}

pub(super) fn write(target: &str, secret: &[u8]) -> Result<(), CredentialManagerError> {
    let mut target_name = wide_null(Path::new(target));
    let credential = CredentialW {
        flags: 0,
        credential_type: CRED_TYPE_GENERIC,
        target_name: target_name.as_mut_ptr(),
        comment: ptr::null_mut(),
        last_written: FileTime::default(),
        credential_blob_size: secret.len() as Dword,
        credential_blob: secret.as_ptr().cast_mut(),
        persist: CRED_PERSIST_LOCAL_MACHINE,
        attribute_count: 0,
        attributes: ptr::null_mut(),
        target_alias: ptr::null_mut(),
        user_name: ptr::null_mut(),
    };
    let success = unsafe {
        // SAFETY: the target is NUL-terminated UTF-16, the secret slice stays
        // alive throughout this synchronous call, and all unused fields are
        // null where the generic credential contract permits it.
        CredWriteW(&credential, 0)
    };
    if success == 0 {
        Err(last_error())
    } else {
        Ok(())
    }
}

pub(super) fn read(target: &str) -> Result<Vec<u8>, CredentialManagerError> {
    let target_name = wide_null(Path::new(target));
    let mut raw_credential = ptr::null_mut();
    let success = unsafe {
        // SAFETY: target_name is NUL-terminated UTF-16 and raw_credential is
        // a valid output pointer for the synchronous Windows call.
        CredReadW(
            target_name.as_ptr(),
            CRED_TYPE_GENERIC,
            0,
            &mut raw_credential,
        )
    };
    if success == 0 {
        return Err(last_error());
    }
    if raw_credential.is_null() {
        return Err(CredentialManagerError::StoredSecretInvalid);
    }

    let credential = ReturnedCredential {
        raw: raw_credential,
    };
    credential.copy_secret()
}

pub(super) fn delete(target: &str) -> Result<bool, CredentialManagerError> {
    let target_name = wide_null(Path::new(target));
    let success = unsafe {
        // SAFETY: target_name is NUL-terminated UTF-16 and the remaining
        // parameters select the exact generic credential with reserved flags.
        CredDeleteW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0)
    };
    if success != 0 {
        Ok(true)
    } else {
        match last_error() {
            CredentialManagerError::NotFound => Ok(false),
            error => Err(error),
        }
    }
}

struct ReturnedCredential {
    raw: *mut CredentialW,
}

impl ReturnedCredential {
    fn copy_secret(&self) -> Result<Vec<u8>, CredentialManagerError> {
        let credential = unsafe {
            // SAFETY: ReturnedCredential is constructed only from a successful
            // CredReadW call and owns this buffer until Drop calls CredFree.
            self.raw.as_ref()
        }
        .ok_or(CredentialManagerError::StoredSecretInvalid)?;
        let size = credential.credential_blob_size as usize;
        if credential.credential_type != CRED_TYPE_GENERIC
            || !(1..=MAX_ANODREL_SECRET_BYTES).contains(&size)
            || credential.credential_blob.is_null()
        {
            return Err(CredentialManagerError::StoredSecretInvalid);
        }
        let bytes = unsafe {
            // SAFETY: Windows returned a credential whose non-null blob is
            // declared to contain exactly size bytes, already bounded above.
            slice::from_raw_parts(credential.credential_blob, size)
        };
        Ok(bytes.to_vec())
    }
}

impl Drop for ReturnedCredential {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: this object owns one CredReadW allocation. Clearing a
            // valid generic blob happens before CredFree releases that block.
            clear_credential_blob(self.raw);
            CredFree(self.raw.cast());
        }
    }
}

unsafe fn clear_credential_blob(credential: *mut CredentialW) {
    let credential = unsafe {
        // SAFETY: caller provides the still-owned credential returned by
        // CredReadW, before CredFree is called.
        credential.as_mut()
    };
    let Some(credential) = credential else {
        return;
    };
    let size = credential.credential_blob_size as usize;
    if credential.credential_blob.is_null() || size > MAX_WINDOWS_BLOB_BYTES {
        return;
    }
    for index in 0..size {
        unsafe {
            // SAFETY: the CredentialBlob range is part of the one allocation
            // returned by CredReadW. Volatile writes prevent elimination before
            // the subsequent CredFree call.
            credential.credential_blob.add(index).write_volatile(0);
        }
    }
}

fn wide_null(value: &Path) -> Vec<u16> {
    value
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn last_error() -> CredentialManagerError {
    map_status(io::Error::last_os_error().raw_os_error())
}

fn map_status(status: Option<i32>) -> CredentialManagerError {
    match status {
        Some(ERROR_NOT_FOUND) => CredentialManagerError::NotFound,
        Some(ERROR_ACCESS_DENIED) => CredentialManagerError::AccessDenied,
        _ => CredentialManagerError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialManagerError, map_status, wide_null};
    use std::path::Path;

    #[test]
    fn status_mapping_does_not_expose_native_error_codes() {
        assert_eq!(map_status(Some(1168)), CredentialManagerError::NotFound);
        assert_eq!(map_status(Some(5)), CredentialManagerError::AccessDenied);
        assert_eq!(map_status(Some(87)), CredentialManagerError::Unavailable);
    }

    #[test]
    fn target_encoding_adds_one_terminal_nul() {
        let encoded = wide_null(Path::new("Anodrel/v1/org.anodrel.test/token"));
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|unit| **unit == 0).count(), 1);
    }
}
