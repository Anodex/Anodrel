//! Narrow direct WinHTTP binding for one already-authorized HTTPS text fetch.

use std::{ffi::c_void, mem, ptr};

use anodrel_network::{
    MAX_NETWORK_TEXT_BYTES, NetworkTextResponse, NetworkTextResponseError, NetworkUrl,
};

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
const READ_BUFFER_BYTES: usize = 4 * 1024;
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

/// A non-native error category used only inside the Windows adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WindowsNetworkError {
    /// WinHTTP could not complete the bounded request.
    Unavailable,
    /// The received status or body could not satisfy the public text contract.
    ResponseInvalid,
}

pub(super) fn fetch_text(url: &NetworkUrl) -> Result<NetworkTextResponse, WindowsNetworkError> {
    let agent = wide_null("Anodrel/0.1");
    let hostname = wide_null(url.hostname());
    let target = wide_null(url.request_target());
    let verb = wide_null("GET");
    let session = WinHttpHandle::open(&agent)?;
    configure_session(&session)?;
    let connection = WinHttpHandle::connect(&session, &hostname, url.port())?;
    let request = WinHttpHandle::request(&connection, &verb, &target)?;
    request.send()?;
    request.receive()?;
    let status_code = request.status_code()?;
    let text = request.read_bounded_utf8()?;
    NetworkTextResponse::new(status_code, text).map_err(response_error)
}

fn configure_session(session: &WinHttpHandle) -> Result<(), WindowsNetworkError> {
    let timeouts_set = unsafe {
        // SAFETY: session owns one successful synchronous WinHTTP session
        // handle. All four timeout values are positive fixed millisecond
        // bounds and contain no application-derived value.
        WinHttpSetTimeouts(
            session.handle,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
            REQUEST_TIMEOUT_MILLISECONDS,
        )
    };
    if timeouts_set == 0 {
        return Err(WindowsNetworkError::Unavailable);
    }
    set_dword_option(session, WINHTTP_OPTION_DISABLE_FEATURE, DISABLED_FEATURES)?;
    set_dword_option(
        session,
        WINHTTP_OPTION_ENABLE_FEATURE,
        WINHTTP_ENABLE_SSL_REVOCATION,
    )
}

fn set_dword_option(
    handle: &WinHttpHandle,
    option: Dword,
    mut value: Dword,
) -> Result<(), WindowsNetworkError> {
    let configured = unsafe {
        // SAFETY: handle owns a live WinHTTP session. value is writable
        // storage for exactly one DWORD, and the two callers use documented
        // session options with host-fixed feature flags only.
        WinHttpSetOption(
            handle.handle,
            option,
            (&mut value as *mut Dword).cast(),
            mem::size_of::<Dword>() as Dword,
        )
    };
    if configured == 0 {
        Err(WindowsNetworkError::Unavailable)
    } else {
        Ok(())
    }
}

struct WinHttpHandle {
    handle: Handle,
}

impl WinHttpHandle {
    fn open(agent: &[u16]) -> Result<Self, WindowsNetworkError> {
        let handle = unsafe {
            // SAFETY: agent is one fixed NUL-terminated UTF-16 user-agent.
            // The direct no-proxy access type and null proxy arguments prevent
            // proxy selection and automatic proxy discovery.
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

    fn connect(session: &Self, hostname: &[u16], port: u16) -> Result<Self, WindowsNetworkError> {
        let handle = unsafe {
            // SAFETY: session owns a live WinHTTP session; hostname is the
            // prevalidated, NUL-terminated DNS name and port comes from the
            // validated URL after exact host-policy matching.
            WinHttpConnect(session.handle, hostname.as_ptr(), port, 0)
        };
        Self::from_handle(handle)
    }

    fn request(
        connection: &Self,
        verb: &[u16],
        target: &[u16],
    ) -> Result<Self, WindowsNetworkError> {
        let handle = unsafe {
            // SAFETY: connection owns a live WinHTTP connection, verb and
            // target are fixed or validated NUL-terminated UTF-16 values, and
            // all omitted arguments deny a referrer and accept-type selector.
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

    fn send(&self) -> Result<(), WindowsNetworkError> {
        let sent = unsafe {
            // SAFETY: self owns a live request. Null header and body pointers
            // with zero lengths send no application-selected header or data;
            // the zero context creates no callback identity.
            WinHttpSendRequest(self.handle, ptr::null(), 0, ptr::null_mut(), 0, 0, 0)
        };
        if sent == 0 {
            Err(WindowsNetworkError::Unavailable)
        } else {
            Ok(())
        }
    }

    fn receive(&self) -> Result<(), WindowsNetworkError> {
        let received = unsafe {
            // SAFETY: self owns the exact request successfully sent above.
            // WinHTTP documents a null reserved argument for synchronous
            // response receipt.
            WinHttpReceiveResponse(self.handle, ptr::null_mut())
        };
        if received == 0 {
            Err(WindowsNetworkError::Unavailable)
        } else {
            Ok(())
        }
    }

    fn status_code(&self) -> Result<u16, WindowsNetworkError> {
        let mut status = 0_u32;
        let mut length = mem::size_of::<Dword>() as Dword;
        let queried = unsafe {
            // SAFETY: self owns a request with a received response. status is
            // writable DWORD storage, length describes it exactly, and no
            // response-header name or index is supplied or retained.
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
            return Err(WindowsNetworkError::Unavailable);
        }
        u16::try_from(status)
            .ok()
            .filter(|status| (100..=599).contains(status))
            .ok_or(WindowsNetworkError::ResponseInvalid)
    }

    fn read_bounded_utf8(&self) -> Result<String, WindowsNetworkError> {
        let mut output = Vec::with_capacity(READ_BUFFER_BYTES);
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        loop {
            let mut available = 0_u32;
            let queried = unsafe {
                // SAFETY: self owns a request with a received response, and
                // available is writable DWORD storage for WinHTTP's count.
                WinHttpQueryDataAvailable(self.handle, &mut available)
            };
            if queried == 0 {
                return Err(WindowsNetworkError::Unavailable);
            }
            if available == 0 {
                break;
            }
            let remaining = MAX_NETWORK_TEXT_BYTES.saturating_sub(output.len());
            if remaining == 0 {
                return Err(WindowsNetworkError::ResponseInvalid);
            }
            let requested = usize::try_from(available)
                .unwrap_or(usize::MAX)
                .min(remaining)
                .min(buffer.len());
            let mut read = 0_u32;
            let read_ok = unsafe {
                // SAFETY: self owns a live request and buffer is writable for
                // exactly requested bytes. read is writable DWORD storage;
                // this synchronous request has no callback or overlapped data.
                WinHttpReadData(
                    self.handle,
                    buffer.as_mut_ptr().cast(),
                    requested as Dword,
                    &mut read,
                )
            };
            if read_ok == 0 || read == 0 || read as usize > requested {
                return Err(WindowsNetworkError::Unavailable);
            }
            output.extend_from_slice(&buffer[..read as usize]);
            if output.len() > MAX_NETWORK_TEXT_BYTES {
                return Err(WindowsNetworkError::ResponseInvalid);
            }
        }
        String::from_utf8(output).map_err(|_| WindowsNetworkError::ResponseInvalid)
    }

    fn from_handle(handle: Handle) -> Result<Self, WindowsNetworkError> {
        if handle.is_null() {
            Err(WindowsNetworkError::Unavailable)
        } else {
            Ok(Self { handle })
        }
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: this guard is constructed only from one successful
            // WinHTTP creator call, and its field is never copied or released
            // elsewhere. Declaration order makes request, connection, then
            // session close in reverse dependency order.
            let _ = WinHttpCloseHandle(self.handle);
        }
    }
}

fn response_error(_: NetworkTextResponseError) -> WindowsNetworkError {
    WindowsNetworkError::ResponseInvalid
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
