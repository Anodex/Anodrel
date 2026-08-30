//! Small Linux syscall boundary for the fixed Wayland client.

use std::{
    ffi::{CStr, c_char, c_int, c_void},
    io,
    mem::{MaybeUninit, size_of, size_of_val},
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    ptr,
};

const AF_UNIX: c_int = 1;
const SOCK_STREAM: c_int = 1;
const SOCK_CLOEXEC: c_int = 0o2000000;
const SOL_SOCKET: c_int = 1;
const SCM_RIGHTS: c_int = 1;
const MSG_CTRUNC: c_int = 0x8;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_SHARED: c_int = 1;
const MFD_CLOEXEC: u32 = 0x1;
const EINTR: i32 = 4;

#[repr(C)]
struct SockAddrUn {
    family: u16,
    path: [c_char; 108],
}

#[repr(C)]
struct IoVec {
    base: *mut c_void,
    length: usize,
}

#[repr(C)]
struct MsgHdr {
    name: *mut c_void,
    name_length: u32,
    iov: *mut IoVec,
    iov_length: usize,
    control: *mut c_void,
    control_length: usize,
    flags: c_int,
}

#[repr(C)]
struct CMsgHdr {
    length: usize,
    level: c_int,
    kind: c_int,
}

unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn connect(socket: c_int, address: *const c_void, length: u32) -> c_int;
    fn write(fd: c_int, buffer: *const c_void, length: usize) -> isize;
    fn recvmsg(fd: c_int, message: *mut MsgHdr, flags: c_int) -> isize;
    fn sendmsg(fd: c_int, message: *const MsgHdr, flags: c_int) -> isize;
    fn close(fd: c_int) -> c_int;
    fn memfd_create(name: *const c_char, flags: u32) -> c_int;
    fn ftruncate(fd: c_int, length: i64) -> c_int;
    fn mmap(
        address: *mut c_void,
        length: usize,
        protection: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(address: *mut c_void, length: usize) -> c_int;
}

/// Opaque local Wayland stream with no native descriptor surface.
pub(super) struct Connection {
    fd: OwnedFd,
}

impl Connection {
    pub(super) fn connect(path: &CStr) -> io::Result<Self> {
        let descriptor = unsafe { socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0) };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(descriptor) };
        let mut address = MaybeUninit::<SockAddrUn>::zeroed();
        let bytes = path.to_bytes_with_nul();
        let address = unsafe { address.assume_init_mut() };
        address.family = AF_UNIX as u16;
        for (target, source) in address.path.iter_mut().zip(bytes) {
            *target = *source as c_char;
        }
        let length = u32::try_from(size_of::<u16>() + bytes.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "Wayland address is invalid")
        })?;
        let result = unsafe {
            connect(
                fd.as_raw_fd(),
                (address as *const SockAddrUn).cast(),
                length,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    pub(super) fn send(&self, bytes: &[u8]) -> io::Result<()> {
        write_all(self.fd.as_raw_fd(), bytes)
    }

    pub(super) fn send_descriptor(&self, bytes: &[u8], descriptor: RawFd) -> io::Result<()> {
        let mut iov = IoVec {
            base: bytes.as_ptr().cast_mut().cast(),
            length: bytes.len(),
        };
        let mut control = [0_usize; 3];
        let header_size = align(size_of::<CMsgHdr>());
        let payload_size = size_of::<RawFd>();
        let total_size = header_size + payload_size;
        let control_bytes = control.as_mut_ptr().cast::<u8>();
        unsafe {
            let header = control_bytes.cast::<CMsgHdr>();
            (*header).length = total_size;
            (*header).level = SOL_SOCKET;
            (*header).kind = SCM_RIGHTS;
            ptr::write_unaligned(control_bytes.add(header_size).cast::<RawFd>(), descriptor);
        }
        let message = MsgHdr {
            name: ptr::null_mut(),
            name_length: 0,
            iov: &mut iov,
            iov_length: 1,
            control: control.as_mut_ptr().cast(),
            control_length: total_size,
            flags: 0,
        };
        let sent = retry_sendmsg(self.fd.as_raw_fd(), &message)?;
        if sent == 0 || sent > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Wayland socket closed",
            ));
        }
        write_all(self.fd.as_raw_fd(), &bytes[sent..])
    }

    /// Appends received bytes and rejects any compositor-provided descriptor.
    pub(super) fn receive(&self, destination: &mut Vec<u8>) -> io::Result<()> {
        let mut bytes = [0_u8; 4096];
        let mut control = [0_usize; 32];
        let mut iov = IoVec {
            base: bytes.as_mut_ptr().cast(),
            length: bytes.len(),
        };
        let mut message = MsgHdr {
            name: ptr::null_mut(),
            name_length: 0,
            iov: &mut iov,
            iov_length: 1,
            control: control.as_mut_ptr().cast(),
            control_length: size_of_val(&control),
            flags: 0,
        };
        let received = retry_recvmsg(self.fd.as_raw_fd(), &mut message)?;
        if received == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Wayland socket closed",
            ));
        }
        let descriptors = close_received_descriptors(&control, message.control_length)?;
        if message.flags & MSG_CTRUNC != 0 || descriptors {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Wayland compositor sent an unexpected descriptor",
            ));
        }
        destination.extend_from_slice(&bytes[..received]);
        Ok(())
    }
}

/// One mapped shared-memory region and its close-on-exec descriptor.
pub(super) struct SharedMemory {
    mapping: *mut u8,
    length: usize,
    descriptor: Option<OwnedFd>,
}

impl SharedMemory {
    pub(super) fn create(length: usize) -> io::Result<Self> {
        if length == 0 || length > i64::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "shared memory size is invalid",
            ));
        }
        let name = b"anodrel-wayland.v1\0";
        let raw = unsafe { memfd_create(name.as_ptr().cast(), MFD_CLOEXEC) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
        if unsafe { ftruncate(descriptor.as_raw_fd(), length as i64) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mapping = unsafe {
            mmap(
                ptr::null_mut(),
                length,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                descriptor.as_raw_fd(),
                0,
            )
        };
        if mapping as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            mapping: mapping.cast(),
            length,
            descriptor: Some(descriptor),
        })
    }

    pub(super) fn descriptor(&self) -> io::Result<RawFd> {
        self.descriptor
            .as_ref()
            .map(AsRawFd::as_raw_fd)
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "shared memory is closed"))
    }

    pub(super) fn close_descriptor(&mut self) {
        self.descriptor.take();
    }

    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.mapping, self.length) }
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        let _ = unsafe { munmap(self.mapping.cast(), self.length) };
    }
}

fn write_all(fd: RawFd, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let written = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
        if written < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(EINTR) {
                continue;
            }
            return Err(error);
        }
        if written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Wayland socket closed",
            ));
        }
        bytes = &bytes[written as usize..];
    }
    Ok(())
}

fn retry_sendmsg(fd: RawFd, message: &MsgHdr) -> io::Result<usize> {
    loop {
        let written = unsafe { sendmsg(fd, message, 0) };
        if written >= 0 {
            return Ok(written as usize);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(EINTR) {
            return Err(error);
        }
    }
}

fn retry_recvmsg(fd: RawFd, message: &mut MsgHdr) -> io::Result<usize> {
    loop {
        let received = unsafe { recvmsg(fd, message, 0) };
        if received >= 0 {
            return Ok(received as usize);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(EINTR) {
            return Err(error);
        }
    }
}

fn close_received_descriptors(control: &[usize], length: usize) -> io::Result<bool> {
    let bytes = control.as_ptr().cast::<u8>();
    let capacity = size_of_val(control);
    if length > capacity {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Wayland control data is invalid",
        ));
    }
    let mut offset = 0;
    let mut found = false;
    while offset < length {
        if length - offset < size_of::<CMsgHdr>() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Wayland control data is invalid",
            ));
        }
        let header = unsafe { ptr::read_unaligned(bytes.add(offset).cast::<CMsgHdr>()) };
        let aligned_header = align(size_of::<CMsgHdr>());
        if header.length < aligned_header || header.length > length - offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Wayland control data is invalid",
            ));
        }
        if header.level == SOL_SOCKET && header.kind == SCM_RIGHTS {
            let payload = header.length - aligned_header;
            if !payload.is_multiple_of(size_of::<RawFd>()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Wayland control data is invalid",
                ));
            }
            for index in (0..payload).step_by(size_of::<RawFd>()) {
                let fd = unsafe {
                    ptr::read_unaligned(bytes.add(offset + aligned_header + index).cast::<RawFd>())
                };
                if fd >= 0 {
                    let _ = unsafe { close(fd) };
                }
            }
            found = payload > 0;
        }
        offset = offset.checked_add(align(header.length)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Wayland control data is invalid",
            )
        })?;
    }
    Ok(found)
}

const fn align(value: usize) -> usize {
    (value + size_of::<usize>() - 1) & !(size_of::<usize>() - 1)
}
