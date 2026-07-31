//! Logon-session-restricted named-pipe security descriptor construction.

use std::{io, mem, ptr, slice};

use crate::raw::{SecurityAttributes, free_local};

const SDDL_REVISION_1: u32 = 1;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_GROUPS: u32 = 2;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const SE_GROUP_LOGON_ID: u32 = 0xC000_0000;
// FILE_GENERIC_READ plus the individual FILE_GENERIC_WRITE rights other than
// FILE_APPEND_DATA/FILE_CREATE_PIPE_INSTANCE. The client needs the attribute
// and READ_CONTROL bits carried by generic pipe access, but never pipe-instance
// creation authority.
const PIPE_CLIENT_ACCESS_MASK: u32 = 0x0012_019B;

#[repr(C)]
struct SidAndAttributes {
    sid: *mut core::ffi::c_void,
    attributes: u32,
}

#[repr(C)]
struct TokenGroups {
    group_count: u32,
    groups: [SidAndAttributes; 1],
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(process: isize, desired_access: u32, token: *mut isize) -> i32;
    fn GetTokenInformation(
        token: isize,
        information_class: u32,
        token_information: *mut core::ffi::c_void,
        token_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *const core::ffi::c_void, string_sid: *mut *mut u16) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn CloseHandle(handle: isize) -> i32;
}

pub struct CurrentSessionSecurity {
    descriptor: *mut core::ffi::c_void,
    attributes: SecurityAttributes,
}

impl CurrentSessionSecurity {
    pub fn new() -> io::Result<Self> {
        let logon_sid = current_logon_sid()?;
        let descriptor_text = wide_null(&format!(
            "D:P(A;;0x{PIPE_CLIENT_ACCESS_MASK:08x};;;{logon_sid})"
        ));
        let mut descriptor = ptr::null_mut();
        // SAFETY: descriptor_text is a null-terminated UTF-16 SDDL string and
        // descriptor receives the LocalAlloc-owned security descriptor.
        let succeeded = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor_text.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        };
        if succeeded == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            descriptor,
            attributes: SecurityAttributes {
                nLength: mem::size_of::<SecurityAttributes>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            },
        })
    }

    pub const fn attributes(&self) -> &SecurityAttributes {
        &self.attributes
    }
}

impl Drop for CurrentSessionSecurity {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            free_local(self.descriptor);
        }
    }
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        string_security_descriptor: *const u16,
        string_sd_revision: u32,
        security_descriptor: *mut *mut core::ffi::c_void,
        security_descriptor_size: *mut u32,
    ) -> i32;
}

fn current_logon_sid() -> io::Result<String> {
    let mut token = 0;
    // SAFETY: GetCurrentProcess returns a pseudo-handle for this process;
    // token points to writable storage for the requested query handle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = logon_sid_from_token(token);
    // SAFETY: OpenProcessToken returned a real token handle owned by this
    // function and it is no longer used after the query completes.
    unsafe {
        CloseHandle(token);
    }
    result
}

fn logon_sid_from_token(token: isize) -> io::Result<String> {
    let mut required = 0;
    // SAFETY: the first call deliberately supplies no output buffer to obtain
    // the size required for the TokenGroups information class.
    let first_succeeded =
        unsafe { GetTokenInformation(token, TOKEN_GROUPS, ptr::null_mut(), 0, &mut required) };
    if first_succeeded != 0
        || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER)
    {
        return Err(io::Error::last_os_error());
    }
    let words = (required as usize).div_ceil(mem::size_of::<usize>());
    let mut storage = vec![0_usize; words];
    // SAFETY: storage is aligned for TokenGroups and has at least required
    // bytes. The API writes only the byte count supplied.
    if unsafe {
        GetTokenInformation(
            token,
            TOKEN_GROUPS,
            storage.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: GetTokenInformation initialized a TokenGroups structure in the
    // aligned storage. Its group count is bounded by the returned byte count.
    let groups = unsafe { &*storage.as_ptr().cast::<TokenGroups>() };
    let groups_offset = mem::offset_of!(TokenGroups, groups);
    let available_groups =
        ((required as usize).saturating_sub(groups_offset)) / mem::size_of::<SidAndAttributes>();
    let group_count = usize::try_from(groups.group_count).unwrap_or(usize::MAX);
    if group_count > available_groups {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "token groups are malformed",
        ));
    }
    // SAFETY: the bounds check above ensures all requested group entries fit in
    // the initialized TokenGroups output buffer.
    let entries = unsafe { slice::from_raw_parts(groups.groups.as_ptr(), group_count) };
    let logon_sid = entries
        .iter()
        .find(|entry| entry.attributes & SE_GROUP_LOGON_ID == SE_GROUP_LOGON_ID)
        .map(|entry| entry.sid)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "process token has no logon SID",
            )
        })?;
    sid_to_string(logon_sid)
}

fn sid_to_string(sid: *mut core::ffi::c_void) -> io::Result<String> {
    let mut string_sid = ptr::null_mut();
    // SAFETY: sid comes from the OS-owned TokenGroups structure and string_sid
    // receives the LocalAlloc-owned null-terminated UTF-16 result.
    if unsafe { ConvertSidToStringSidW(sid, &mut string_sid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut length = 0;
    // SAFETY: ConvertSidToStringSidW guarantees a null-terminated UTF-16 string.
    unsafe {
        while *string_sid.add(length) != 0 {
            length += 1;
        }
    }
    // SAFETY: length was measured to the null terminator in the allocation.
    let value = unsafe { String::from_utf16(slice::from_raw_parts(string_sid, length)) }
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "logon SID is not UTF-16"));
    free_local(string_sid.cast());
    value
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
