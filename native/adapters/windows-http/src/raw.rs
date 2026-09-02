//! Direct WinHTTP FFI and RAII handle ownership for one streaming `GET`.

use std::{ffi::c_void, mem, ptr};

use anodrel_network::NetworkOrigin;

use crate::WindowsHttpsError;

type Bool = i32;
type Dword = u32;
type Handle = *mut c_void;

const WINHTTP_ACCESS_TYPE_NO_PROXY: Dword = 1;
const WINHTTP_FLAG_SECURE: Dword = 0x0080_0000;
const WINHTTP_OPTION_DISABLE_FEATURE: Dword = 63;
const WINHTTP_OPTION_ENABLE_FEATURE: Dword = 79;
const WINHTTP_DISABLE_COOKIES: Dword = 0x0000_0001;
const WINHTTP_DISABLE_REDIRECTS: Dword = 0x0000_0002;
const WINHTTP_DISABLE_AUTHENTICATION: Dword = 0x0000_0004;
const WINHTTP_DISABLE_KEEP_ALIVE: Dword = 0x0000_0008;
const WINHTTP_ENABLE_SSL_REVOCATION: Dword = 0x0000_0001;
const WINHTTP_QUERY_STATUS_CODE: Dword = 19;
const WINHTTP_QUERY_FLAG_NUMBER: Dword = 0x2000_0000;
const REQUEST_TIMEOUT_MILLISECONDS: i32 = 10_000;
const READ_BUFFER_BYTES: usize = 64 * 1024;
const DISABLED_FEATURES: Dword = WINHTTP_DISABLE_COOKIES
    | WINHTTP_DISABLE_REDIRECTS
    | WINHTTP_DISABLE_AUTHENTICATION
    | WINHTTP_DISABLE_KEEP_ALIVE;

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        agent: *const u16,
        access_type: Dword,
        proxy: *const u16,
        proxy_bypass: *const u16,
        flags: Dword,
    ) -> Handle;
    fn WinHttpCloseHandle(handle: Handle) -> Bool;
    fn WinHttpConnect(
        session: Handle,
        server_name: *const u16,
        server_port: u16,
        reserved: Dword,
    ) -> Handle;
    fn WinHttpOpenRequest(
        connection: Handle,
        verb: *const u16,
        object_name: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: Dword,
    ) -> Handle;
    fn WinHttpSendRequest(
        request: Handle,
        headers: *const u16,
        headers_length: Dword,
        optional_data: *mut c_void,
        optional_length: Dword,
        total_length: Dword,
        context: usize,
    ) -> Bool;
    fn WinHttpReceiveResponse(request: Handle, reserved: *mut c_void) -> Bool;
    fn WinHttpQueryHeaders(
        request: Handle,
        info_level: Dword,
        name: *const u16,
        buffer: *mut c_void,
        buffer_length: *mut Dword,
        index: *mut Dword,
    ) -> Bool;
    fn WinHttpQueryDataAvailable(request: Handle, available: *mut Dword) -> Bool;
    fn WinHttpReadData(
        request: Handle,
        buffer: *mut c_void,
        bytes_to_read: Dword,
        bytes_read: *mut Dword,
    ) -> Bool;
    fn WinHttpSetOption(
        handle: Handle,
        option: Dword,
        buffer: *mut c_void,
        buffer_length: Dword,
    ) -> Bool;
    fn WinHttpSetTimeouts(
        handle: Handle,
        resolve_timeout: i32,
        connect_timeout: i32,
        send_timeout: i32,
        receive_timeout: i32,
    ) -> Bool;
}

/// Transfers one bounded response to the caller's chunk consumer.
pub(super) fn get(
    origin: &NetworkOrigin,
    request_target: &str,
    expected_status: Option<u16>,
    maximum_body_bytes: usize,
    consumer: &mut dyn FnMut(&[u8]) -> Result<(), ()>,
) -> Result<u16, WindowsHttpsError> {
    let agent = wide_null("Anodrel/0.1");
    let hostname = wide_null(origin.hostname());
    let target = wide_null(request_target);
    let verb = wide_null("GET");
    let session = WinHttpHandle::open(&agent)?;
    configure_session(&session)?;
    let connection = WinHttpHandle::connect(&session, &hostname, origin.port())?;
    let request = WinHttpHandle::request(&connection, &verb, &target)?;
    configure_request(&request)?;
    request.send()?;
    request.receive()?;
    let status = request.status_code()?;
    if expected_status.is_some_and(|expected| expected != status) {
        return Err(WindowsHttpsError::UnexpectedStatus);
    }
    request.read_bounded(maximum_body_bytes, consumer)?;
    Ok(status)
}

fn configure_session(session: &WinHttpHandle) -> Result<(), WindowsHttpsError> {
    // SAFETY: session owns one successful synchronous WinHTTP session handle.
    // All four timeout values are positive fixed millisecond bounds and contain
    // no request-derived value.
    let set = unsafe {
        WinHttpSetTimeouts(
            session.handle,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
        )
    };
    (set != 0)
        .then_some(())
        .ok_or(WindowsHttpsError::Unavailable)
}

fn configure_request(request: &WinHttpHandle) -> Result<(), WindowsHttpsError> {
    set_dword_option(request, WINHTTP_OPTION_DISABLE_FEATURE, DISABLED_FEATURES)?;
    set_dword_option(
        request,
        WINHTTP_OPTION_ENABLE_FEATURE,
        WINHTTP_ENABLE_SSL_REVOCATION,
    )
}

fn set_dword_option(
    handle: &WinHttpHandle,
    option: Dword,
    mut value: Dword,
) -> Result<(), WindowsHttpsError> {
    // SAFETY: handle owns a live WinHTTP handle. value is writable storage for
    // exactly one DWORD, and each caller uses a documented fixed feature option
    // at the required handle level.
    let set = unsafe {
        WinHttpSetOption(
            handle.handle,
            option,
            (&mut value as *mut Dword).cast(),
            mem::size_of::<Dword>() as Dword,
        )
    };
    (set != 0)
        .then_some(())
        .ok_or(WindowsHttpsError::Unavailable)
}

struct WinHttpHandle {
    handle: Handle,
}

impl WinHttpHandle {
    fn open(agent: &[u16]) -> Result<Self, WindowsHttpsError> {
        // SAFETY: agent is one fixed NUL-terminated UTF-16 user-agent. The
        // direct no-proxy access type and null proxy arguments prevent proxy
        // selection and automatic proxy discovery.
        let handle = unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_NO_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        };
        Self::from_handle(handle)
    }

    fn connect(session: &Self, hostname: &[u16], port: u16) -> Result<Self, WindowsHttpsError> {
        // SAFETY: session owns a live WinHTTP session; hostname and port come
        // only from a validated exact origin and remain live through the call.
        let handle = unsafe { WinHttpConnect(session.handle, hostname.as_ptr(), port, 0) };
        Self::from_handle(handle)
    }

    fn request(connection: &Self, verb: &[u16], target: &[u16]) -> Result<Self, WindowsHttpsError> {
        // SAFETY: connection owns a live WinHTTP connection. The verb is fixed
        // and target is a prevalidated NUL-terminated request target; omitted
        // fields deny a referrer and accept-type selector.
        let handle = unsafe {
            WinHttpOpenRequest(
                connection.handle,
                verb.as_ptr(),
                target.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        };
        Self::from_handle(handle)
    }

    fn send(&self) -> Result<(), WindowsHttpsError> {
        // SAFETY: self owns a live request. Null header and body pointers with
        // zero lengths send no caller-selected header or data, and zero context
        // creates no callback identity.
        let sent =
            unsafe { WinHttpSendRequest(self.handle, ptr::null(), 0, ptr::null_mut(), 0, 0, 0) };
        (sent != 0)
            .then_some(())
            .ok_or(WindowsHttpsError::Unavailable)
    }

    fn receive(&self) -> Result<(), WindowsHttpsError> {
        // SAFETY: self owns the exact request successfully sent above. WinHTTP
        // documents a null reserved argument for synchronous response receipt.
        let received = unsafe { WinHttpReceiveResponse(self.handle, ptr::null_mut()) };
        (received != 0)
            .then_some(())
            .ok_or(WindowsHttpsError::Unavailable)
    }

    fn status_code(&self) -> Result<u16, WindowsHttpsError> {
        let mut status = 0_u32;
        let mut length = mem::size_of::<Dword>() as Dword;
        // SAFETY: self owns a request with a received response. status is one
        // writable DWORD, and no response-header name or index is retained.
        let queried = unsafe {
            WinHttpQueryHeaders(
                self.handle,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                ptr::null(),
                (&mut status as *mut Dword).cast(),
                &mut length,
                ptr::null_mut(),
            )
        };
        if queried == 0 || length != mem::size_of::<Dword>() as Dword {
            return Err(WindowsHttpsError::Unavailable);
        }
        u16::try_from(status)
            .ok()
            .filter(|status| (100..=599).contains(status))
            .ok_or(WindowsHttpsError::ResponseInvalid)
    }

    fn read_bounded(
        &self,
        maximum_body_bytes: usize,
        consumer: &mut dyn FnMut(&[u8]) -> Result<(), ()>,
    ) -> Result<(), WindowsHttpsError> {
        let mut total = 0_usize;
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        loop {
            let mut available = 0_u32;
            // SAFETY: self owns a request with a received response, and
            // available is writable DWORD storage for WinHTTP's byte count.
            let queried = unsafe { WinHttpQueryDataAvailable(self.handle, &mut available) };
            if queried == 0 {
                return Err(WindowsHttpsError::Unavailable);
            }
            if available == 0 {
                return Ok(());
            }
            let remaining = maximum_body_bytes.saturating_sub(total);
            if remaining == 0 {
                return Err(WindowsHttpsError::BodyTooLarge);
            }
            let requested = usize::try_from(available)
                .unwrap_or(usize::MAX)
                .min(remaining)
                .min(buffer.len());
            let mut read = 0_u32;
            // SAFETY: self owns a live request and buffer is writable for
            // exactly `requested` bytes. read is writable DWORD storage and
            // this synchronous request has no callback or overlapped data.
            let read_ok = unsafe {
                WinHttpReadData(
                    self.handle,
                    buffer.as_mut_ptr().cast(),
                    requested as Dword,
                    &mut read,
                )
            };
            if read_ok == 0 || read == 0 || read as usize > requested {
                return Err(WindowsHttpsError::Unavailable);
            }
            total = total
                .checked_add(read as usize)
                .ok_or(WindowsHttpsError::BodyTooLarge)?;
            if total > maximum_body_bytes {
                return Err(WindowsHttpsError::BodyTooLarge);
            }
            consumer(&buffer[..read as usize]).map_err(|_| WindowsHttpsError::ConsumerRejected)?;
        }
    }

    fn from_handle(handle: Handle) -> Result<Self, WindowsHttpsError> {
        (!handle.is_null())
            .then_some(Self { handle })
            .ok_or(WindowsHttpsError::Unavailable)
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        // SAFETY: this guard is constructed only from a successful WinHTTP
        // creator call, and its field is never copied or released elsewhere.
        // Declaration order closes request, connection, then session.
        let _ = unsafe { WinHttpCloseHandle(self.handle) };
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        DISABLED_FEATURES, WINHTTP_DISABLE_AUTHENTICATION, WINHTTP_DISABLE_COOKIES,
        WINHTTP_DISABLE_KEEP_ALIVE, WINHTTP_DISABLE_REDIRECTS, wide_null,
    };

    #[test]
    fn fixed_windows_strings_have_one_terminal_nul() {
        let encoded = wide_null("/v1/status?format=text%2Fplain");
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|unit| **unit == 0).count(), 1);
    }

    #[test]
    fn request_disables_every_stateful_or_automatic_feature() {
        assert_ne!(DISABLED_FEATURES & WINHTTP_DISABLE_COOKIES, 0);
        assert_ne!(DISABLED_FEATURES & WINHTTP_DISABLE_REDIRECTS, 0);
        assert_ne!(DISABLED_FEATURES & WINHTTP_DISABLE_AUTHENTICATION, 0);
        assert_ne!(DISABLED_FEATURES & WINHTTP_DISABLE_KEEP_ALIVE, 0);
    }
}
