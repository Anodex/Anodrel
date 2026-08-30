//! Direct effective-account lookup through the Linux C-library interface.

use std::{ffi::c_char, os::unix::ffi::OsStrExt, path::PathBuf, ptr};

const INITIAL_BUFFER_BYTES: usize = 4 * 1024;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;
const ERANGE: i32 = 34;

type Uid = u32;

#[repr(C)]
struct Passwd {
    name: *mut c_char,
    password: *mut c_char,
    uid: Uid,
    gid: u32,
    gecos: *mut c_char,
    directory: *mut c_char,
    shell: *mut c_char,
}

#[link(name = "c")]
unsafe extern "C" {
    fn geteuid() -> Uid;
    fn getpwuid_r(
        uid: Uid,
        record: *mut Passwd,
        buffer: *mut c_char,
        buffer_length: usize,
        result: *mut *mut Passwd,
    ) -> i32;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AccountLookupError {
    Unavailable,
    InvalidHome,
}

pub(super) fn effective_home_directory() -> Result<PathBuf, AccountLookupError> {
    // SAFETY: geteuid has no arguments, does not allocate, and returns this
    // process's effective UID without exposing it outside this adapter.
    let uid = unsafe { geteuid() };
    lookup_home_directory(uid)
}

fn lookup_home_directory(uid: Uid) -> Result<PathBuf, AccountLookupError> {
    let mut buffer = vec![0_u8; INITIAL_BUFFER_BYTES];
    loop {
        let mut record = empty_record();
        let mut result = ptr::null_mut();
        // SAFETY: record and result are writable storage. The byte buffer is
        // allocated for exactly buffer_length bytes and remains live for the
        // reentrant lookup. The effective UID is a plain Linux value.
        let status = unsafe {
            getpwuid_r(
                uid,
                &raw mut record,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &raw mut result,
            )
        };
        if status == 0 {
            let home = if result.is_null() {
                Err(AccountLookupError::Unavailable)
            } else {
                home_from_record(&record, &buffer)
            };
            buffer.fill(0);
            return home;
        }
        if status == ERANGE
            && let Some(next) = next_buffer_bytes(buffer.len())
        {
            buffer.fill(0);
            buffer.resize(next, 0);
            continue;
        }
        buffer.fill(0);
        return Err(AccountLookupError::Unavailable);
    }
}

fn empty_record() -> Passwd {
    Passwd {
        name: ptr::null_mut(),
        password: ptr::null_mut(),
        uid: 0,
        gid: 0,
        gecos: ptr::null_mut(),
        directory: ptr::null_mut(),
        shell: ptr::null_mut(),
    }
}

fn home_from_record(record: &Passwd, buffer: &[u8]) -> Result<PathBuf, AccountLookupError> {
    let bytes =
        c_string_inside_buffer(record.directory, buffer).ok_or(AccountLookupError::InvalidHome)?;
    if bytes.is_empty() {
        return Err(AccountLookupError::InvalidHome);
    }
    let home = PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
    if home.is_absolute() {
        Ok(home)
    } else {
        Err(AccountLookupError::InvalidHome)
    }
}

fn c_string_inside_buffer(value: *const c_char, buffer: &[u8]) -> Option<&[u8]> {
    if value.is_null() {
        return None;
    }
    let start = buffer.as_ptr() as usize;
    let end = start.checked_add(buffer.len())?;
    let value = value.cast::<u8>() as usize;
    if value < start || value >= end {
        return None;
    }
    let bytes = &buffer[value - start..];
    let terminator = bytes.iter().position(|byte| *byte == 0)?;
    Some(&bytes[..terminator])
}

const fn next_buffer_bytes(current: usize) -> Option<usize> {
    match current.checked_mul(2) {
        Some(next) if next <= MAX_BUFFER_BYTES => Some(next),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        AccountLookupError, Passwd, c_string_inside_buffer, home_from_record, next_buffer_bytes,
    };

    #[test]
    fn bounded_growth_stops_at_the_documented_account_record_limit() {
        assert_eq!(next_buffer_bytes(4 * 1024), Some(8 * 1024));
        assert_eq!(next_buffer_bytes(512 * 1024), Some(1024 * 1024));
        assert_eq!(next_buffer_bytes(1024 * 1024), None);
    }

    #[test]
    fn a_home_pointer_must_stay_inside_the_reentrant_buffer() {
        let mut buffer = b"/home/anodrel\0ignored".to_vec();
        let record = Passwd {
            directory: buffer.as_mut_ptr().cast(),
            ..super::empty_record()
        };
        assert_eq!(
            home_from_record(&record, &buffer).expect("absolute home is accepted"),
            Path::new("/home/anodrel")
        );
        let outside = Passwd {
            directory: c"/outside".as_ptr().cast_mut(),
            ..super::empty_record()
        };
        assert_eq!(
            home_from_record(&outside, &buffer),
            Err(AccountLookupError::InvalidHome)
        );
        assert_eq!(c_string_inside_buffer(std::ptr::null(), &buffer), None);
    }
}
