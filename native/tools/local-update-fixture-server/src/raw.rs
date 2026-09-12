//! Narrow direct bindings for the Windows HTTP Server API request queue.

use std::{ffi::c_void, fs::File, mem, os::windows::io::AsRawHandle, ptr};

use anodrel_local_update_fixture::{LOCALHOST, PORT};

const HTTP_INITIALIZE_SERVER: u32 = 1;
const HTTP_VERB_GET: u32 = 4;
const HTTP_REQUEST_FLAG_MORE_ENTITY_BODY_EXISTS: u32 = 1;
const HTTP_HEADER_TRANSFER_ENCODING: usize = 6;
const HTTP_HEADER_CONTENT_LENGTH: usize = 11;
const ERROR_MORE_DATA: u32 = 234;
const REQUEST_BUFFER_U64S: usize = 8_192;
const REQUEST_HEADER_COUNT: usize = 41;
const RESPONSE_HEADER_COUNT: usize = 30;
const RESPONSE_HEADER_CONTENT_LENGTH: usize = 11;
const HTTP_DATA_CHUNK_FROM_FILE_HANDLE: u32 = 1;
const HTTP_BYTE_RANGE_TO_EOF: u64 = u64::MAX;

type Handle = *mut c_void;

#[link(name = "httpapi")]
unsafe extern "system" {
    fn HttpInitialize(version: HttpApiVersion, flags: u32, reserved: *mut c_void) -> u32;
    fn HttpTerminate(flags: u32, reserved: *mut c_void) -> u32;
    fn HttpCreateHttpHandle(queue: *mut Handle, reserved: u32) -> u32;
    fn HttpAddUrl(queue: Handle, url: *const u16, reserved: *mut c_void) -> u32;
    fn HttpRemoveUrl(queue: Handle, url: *const u16) -> u32;
    fn HttpReceiveHttpRequest(
        queue: Handle,
        request_id: u64,
        flags: u32,
        request: *mut HttpRequestV1,
        request_length: u32,
        received: *mut u32,
        overlapped: *mut c_void,
    ) -> u32;
    fn HttpSendHttpResponse(
        queue: Handle,
        request_id: u64,
        flags: u32,
        response: *const HttpResponseV1,
        cache_policy: *const c_void,
        bytes_sent: *mut u32,
        reserved_one: *mut c_void,
        reserved_two: u32,
        overlapped: *mut c_void,
        log_data: *mut c_void,
    ) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: Handle) -> i32;
}

/// A checked incoming request held only until one response is sent.
#[derive(Debug)]
pub(crate) struct ReceivedRequest {
    request_id: u64,
    method: &'static str,
    target: String,
    has_body: bool,
}

impl ReceivedRequest {
    /// Returns the one normalized method token this server recognizes.
    pub(crate) fn method(&self) -> &str {
        self.method
    }

    /// Returns the cooked absolute request target, or an empty string on refusal.
    pub(crate) fn target(&self) -> &str {
        &self.target
    }

    /// Returns whether the request had entity-body evidence.
    pub(crate) fn has_body(&self) -> bool {
        self.has_body
    }
}

/// One initialized and registered fixed HTTP Server API request queue.
pub(crate) struct RequestQueue {
    handle: Handle,
    url: Vec<u16>,
    initialized: bool,
}

impl RequestQueue {
    /// Initializes and binds the one fixed localhost HTTPS prefix.
    pub(crate) fn bind() -> Result<Self, RawError> {
        let version = HttpApiVersion { major: 1, minor: 0 };
        // SAFETY: version and flags are fixed documented HTTP Server API
        // values; no configuration pointer is accepted or retained.
        if unsafe { HttpInitialize(version, HTTP_INITIALIZE_SERVER, ptr::null_mut()) } != 0 {
            return Err(RawError::Unavailable);
        }
        let mut handle = ptr::null_mut();
        // SAFETY: handle is writable storage for the request queue produced by
        // the initialized HTTP Server API. Reserved is documented as zero.
        if unsafe { HttpCreateHttpHandle(&mut handle, 0) } != 0 || handle.is_null() {
            // SAFETY: this balances the successful fixed server initialization.
            let _ = unsafe { HttpTerminate(HTTP_INITIALIZE_SERVER, ptr::null_mut()) };
            return Err(RawError::Unavailable);
        }
        let url = wide_null(&format!("https://{LOCALHOST}:{PORT}/anodrel/local-update/"));
        // SAFETY: handle owns a live request queue and url is NUL-terminated
        // UTF-16 storage that remains owned by the returned queue.
        if unsafe { HttpAddUrl(handle, url.as_ptr(), ptr::null_mut()) } != 0 {
            // SAFETY: handle and initialization were both obtained above.
            let _ = unsafe { CloseHandle(handle) };
            // SAFETY: balances the successful fixed initialization.
            let _ = unsafe { HttpTerminate(HTTP_INITIALIZE_SERVER, ptr::null_mut()) };
            return Err(RawError::Unavailable);
        }
        Ok(Self {
            handle,
            url,
            initialized: true,
        })
    }

    /// Receives one request into a fixed aligned header buffer.
    pub(crate) fn receive(&self) -> Result<ReceivedRequest, RawError> {
        let mut buffer = vec![0_u64; REQUEST_BUFFER_U64S];
        let mut received = 0_u32;
        // SAFETY: the u64 backing storage gives the request structure at least
        // eight-byte alignment and fixed 64 KiB writable capacity. This is a
        // synchronous receive with no callback or overlapped pointer.
        let status = unsafe {
            HttpReceiveHttpRequest(
                self.handle,
                0,
                0,
                buffer.as_mut_ptr().cast(),
                (buffer.len() * mem::size_of::<u64>()) as u32,
                &mut received,
                ptr::null_mut(),
            )
        };
        if status == ERROR_MORE_DATA {
            return Err(RawError::RequestTooLarge);
        }
        if status != 0 || received < mem::size_of::<HttpRequestV1>() as u32 {
            return Err(RawError::Unavailable);
        }
        // SAFETY: the HTTP Server API wrote at least one complete V1 prefix to
        // the aligned buffer. V2 extends V1, so the prefix is valid on current
        // Windows versions. Pointer fields are validated before dereference.
        let request = unsafe { &*buffer.as_ptr().cast::<HttpRequestV1>() };
        Ok(ReceivedRequest {
            request_id: request.request_id,
            method: if request.verb == HTTP_VERB_GET {
                "GET"
            } else {
                ""
            },
            target: cooked_path(request, &buffer).unwrap_or_default(),
            has_body: has_body(request),
        })
    }

    /// Sends the checked file through one synchronous response.
    pub(crate) fn send_file(
        &self,
        request: ReceivedRequest,
        file: &File,
        bytes: u64,
    ) -> Result<(), RawError> {
        send_response(self.handle, request.request_id, Some((file, bytes)))
    }

    /// Sends the same fixed not-found response for every refused route.
    pub(crate) fn send_not_found(&self, request: ReceivedRequest) -> Result<(), RawError> {
        send_response(self.handle, request.request_id, None)
    }
}

impl Drop for RequestQueue {
    fn drop(&mut self) {
        // SAFETY: this queue owns the only registration and handle created in
        // bind. Windows ignores teardown failures while process cleanup closes
        // any remaining native state.
        let _ = unsafe { HttpRemoveUrl(self.handle, self.url.as_ptr()) };
        // SAFETY: the queue handle is live until this one Drop implementation.
        let _ = unsafe { CloseHandle(self.handle) };
        if self.initialized {
            // SAFETY: exactly balances the successful HttpInitialize call.
            let _ = unsafe { HttpTerminate(HTTP_INITIALIZE_SERVER, ptr::null_mut()) };
        }
    }
}

fn send_response(
    queue: Handle,
    request_id: u64,
    file: Option<(&File, u64)>,
) -> Result<(), RawError> {
    let (status, reason) = if file.is_some() {
        (200_u16, b"OK".as_slice())
    } else {
        (404, b"Not Found".as_slice())
    };
    let length_text = file.map_or_else(|| "0".to_owned(), |(_, bytes)| bytes.to_string());
    let mut headers = HttpResponseHeaders::empty();
    headers.known_headers[RESPONSE_HEADER_CONTENT_LENGTH] = HttpKnownHeader {
        raw_value_length: length_text.len() as u16,
        raw_value: length_text.as_ptr(),
    };
    let mut chunk = file.map(|(file, _)| HttpDataChunk {
        kind: HTTP_DATA_CHUNK_FROM_FILE_HANDLE,
        source: HttpDataChunkSource {
            from_file: HttpFromFile {
                byte_range: HttpByteRange {
                    starting_offset: 0,
                    length: HTTP_BYTE_RANGE_TO_EOF,
                },
                handle: file.as_raw_handle().cast(),
            },
        },
    });
    let response = HttpResponseV1 {
        flags: 0,
        version: HttpVersion { major: 1, minor: 1 },
        status_code: status,
        reason_length: reason.len() as u16,
        reason: reason.as_ptr(),
        headers,
        entity_chunk_count: u16::from(chunk.is_some()),
        entity_chunks: chunk.as_mut().map_or(ptr::null_mut(), |chunk| chunk),
    };
    let mut sent = 0_u32;
    // SAFETY: response, fixed reason, content-length text, optional chunk, and
    // file handle all remain live for this synchronous API call. The response
    // has no caller-controlled headers, cache policy, callback, or log data.
    let status = unsafe {
        HttpSendHttpResponse(
            queue,
            request_id,
            0,
            &response,
            ptr::null(),
            &mut sent,
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    (status == 0).then_some(()).ok_or(RawError::Unavailable)
}

fn has_body(request: &HttpRequestV1) -> bool {
    request.flags & HTTP_REQUEST_FLAG_MORE_ENTITY_BODY_EXISTS != 0
        || request.entity_chunk_count != 0
        || request.headers.known_headers[HTTP_HEADER_CONTENT_LENGTH].raw_value_length != 0
        || request.headers.known_headers[HTTP_HEADER_TRANSFER_ENCODING].raw_value_length != 0
}

fn cooked_path(request: &HttpRequestV1, buffer: &[u64]) -> Option<String> {
    let cooked = &request.cooked_url;
    if cooked.query_length != 0
        || cooked.absolute_path.is_null()
        || cooked.absolute_path_length == 0
    {
        return None;
    }
    let units = usize::from(cooked.absolute_path_length).checked_div(2)?;
    let start = cooked.absolute_path as usize;
    let end = start.checked_add(units.checked_mul(mem::size_of::<u16>())?)?;
    let buffer_start = buffer.as_ptr() as usize;
    let buffer_end = buffer_start.checked_add(mem::size_of_val(buffer))?;
    if start < buffer_start || end > buffer_end || !start.is_multiple_of(mem::align_of::<u16>()) {
        return None;
    }
    // SAFETY: bounds and alignment above prove that this HTTP Server API
    // pointer addresses exactly `units` UTF-16 values inside our owned buffer.
    let path = unsafe { std::slice::from_raw_parts(cooked.absolute_path, units) };
    String::from_utf16(path).ok()
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A closed direct HTTP Server API failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RawError {
    /// Windows could not perform the fixed listener operation.
    Unavailable,
    /// A request exceeded the server's fixed bounded header buffer.
    RequestTooLarge,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HttpApiVersion {
    major: u16,
    minor: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HttpVersion {
    major: u16,
    minor: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HttpKnownHeader {
    raw_value_length: u16,
    raw_value: *const u8,
}

const EMPTY_HEADER: HttpKnownHeader = HttpKnownHeader {
    raw_value_length: 0,
    raw_value: ptr::null(),
};

#[repr(C)]
struct HttpRequestHeaders {
    unknown_header_count: u16,
    unknown_headers: *const c_void,
    trailer_count: u16,
    trailers: *const c_void,
    known_headers: [HttpKnownHeader; REQUEST_HEADER_COUNT],
}

#[repr(C)]
struct HttpResponseHeaders {
    unknown_header_count: u16,
    unknown_headers: *const c_void,
    trailer_count: u16,
    trailers: *const c_void,
    known_headers: [HttpKnownHeader; RESPONSE_HEADER_COUNT],
}

impl HttpResponseHeaders {
    const fn empty() -> Self {
        Self {
            unknown_header_count: 0,
            unknown_headers: ptr::null(),
            trailer_count: 0,
            trailers: ptr::null(),
            known_headers: [EMPTY_HEADER; RESPONSE_HEADER_COUNT],
        }
    }
}

#[repr(C)]
struct HttpCookedUrl {
    full_url_length: u16,
    host_length: u16,
    absolute_path_length: u16,
    query_length: u16,
    full_url: *const u16,
    host: *const u16,
    absolute_path: *const u16,
    query: *const u16,
}

#[repr(C)]
struct HttpTransportAddress {
    remote_address: *const c_void,
    local_address: *const c_void,
}

#[repr(C)]
struct HttpRequestV1 {
    flags: u32,
    connection_id: u64,
    request_id: u64,
    url_context: u64,
    version: HttpVersion,
    verb: u32,
    unknown_verb_length: u16,
    raw_url_length: u16,
    unknown_verb: *const u8,
    raw_url: *const u8,
    cooked_url: HttpCookedUrl,
    address: HttpTransportAddress,
    headers: HttpRequestHeaders,
    bytes_received: u64,
    entity_chunk_count: u16,
    entity_chunks: *const c_void,
    raw_connection_id: u64,
    ssl_info: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HttpByteRange {
    starting_offset: u64,
    length: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HttpFromFile {
    byte_range: HttpByteRange,
    handle: Handle,
}

#[repr(C)]
union HttpDataChunkSource {
    from_file: HttpFromFile,
}

#[repr(C)]
struct HttpDataChunk {
    kind: u32,
    source: HttpDataChunkSource,
}

#[repr(C)]
struct HttpResponseV1 {
    flags: u32,
    version: HttpVersion,
    status_code: u16,
    reason_length: u16,
    reason: *const u8,
    headers: HttpResponseHeaders,
    entity_chunk_count: u16,
    entity_chunks: *mut HttpDataChunk,
}

#[cfg(test)]
mod tests {
    use super::{
        HTTP_HEADER_CONTENT_LENGTH, HTTP_HEADER_TRANSFER_ENCODING, HTTP_VERB_GET, HttpCookedUrl,
        HttpKnownHeader, HttpRequestHeaders, HttpRequestV1, HttpTransportAddress, HttpVersion,
        REQUEST_BUFFER_U64S, REQUEST_HEADER_COUNT, cooked_path, has_body, wide_null,
    };

    #[test]
    fn fixed_listener_url_is_nul_terminated() {
        let url = wide_null("https://localhost:45863/anodrel/local-update/");
        assert_eq!(url.last(), Some(&0));
        assert_eq!(url.iter().filter(|unit| **unit == 0).count(), 1);
    }

    #[test]
    fn request_body_evidence_is_refused() {
        let mut request = empty_request();
        assert!(!has_body(&request));
        request.headers.known_headers[HTTP_HEADER_CONTENT_LENGTH].raw_value_length = 1;
        assert!(has_body(&request));
        request.headers.known_headers[HTTP_HEADER_CONTENT_LENGTH].raw_value_length = 0;
        request.headers.known_headers[HTTP_HEADER_TRANSFER_ENCODING].raw_value_length = 1;
        assert!(has_body(&request));
    }

    #[test]
    fn incomplete_cooked_url_is_refused_without_dereferencing_it() {
        let buffer = vec![0_u64; REQUEST_BUFFER_U64S];
        let request = empty_request();
        assert!(cooked_path(&request, &buffer).is_none());
    }

    #[test]
    fn fixed_request_buffer_meets_the_v1_prefix_alignment_and_size_requirements() {
        assert!(std::mem::align_of::<HttpRequestV1>() <= std::mem::align_of::<u64>());
        assert!(std::mem::size_of::<HttpRequestV1>() <= REQUEST_BUFFER_U64S * 8);
    }

    fn empty_request() -> HttpRequestV1 {
        HttpRequestV1 {
            flags: 0,
            connection_id: 0,
            request_id: 0,
            url_context: 0,
            version: HttpVersion { major: 1, minor: 1 },
            verb: HTTP_VERB_GET,
            unknown_verb_length: 0,
            raw_url_length: 0,
            unknown_verb: std::ptr::null(),
            raw_url: std::ptr::null(),
            cooked_url: HttpCookedUrl {
                full_url_length: 0,
                host_length: 0,
                absolute_path_length: 0,
                query_length: 0,
                full_url: std::ptr::null(),
                host: std::ptr::null(),
                absolute_path: std::ptr::null(),
                query: std::ptr::null(),
            },
            address: HttpTransportAddress {
                remote_address: std::ptr::null(),
                local_address: std::ptr::null(),
            },
            headers: HttpRequestHeaders {
                unknown_header_count: 0,
                unknown_headers: std::ptr::null(),
                trailer_count: 0,
                trailers: std::ptr::null(),
                known_headers: [HttpKnownHeader {
                    raw_value_length: 0,
                    raw_value: std::ptr::null(),
                }; REQUEST_HEADER_COUNT],
            },
            bytes_received: 0,
            entity_chunk_count: 0,
            entity_chunks: std::ptr::null(),
            raw_connection_id: 0,
            ssl_info: std::ptr::null(),
        }
    }
}
