//! Direct Linux endpoint generation, abstract socket addressing, and peer checks.

use std::{
    fs::File,
    io::{self, Read},
    os::{
        fd::AsRawFd,
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixStream},
    },
};

const RANDOM_BYTES: usize = 32;
const SOL_SOCKET: i32 = 1;
const SO_PEERCRED: i32 = 17;

type Socklen = u32;

#[repr(C)]
struct UCred {
    pid: i32,
    uid: u32,
    gid: u32,
}

#[link(name = "c")]
unsafe extern "C" {
    fn geteuid() -> u32;
    fn getsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *mut core::ffi::c_void,
        option_length: *mut Socklen,
    ) -> i32;
}

pub(super) fn random_hex() -> io::Result<String> {
    let mut bytes = [0_u8; RANDOM_BYTES];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let mut text = String::with_capacity(RANDOM_BYTES * 2);
    for byte in bytes {
        text.push(hex_digit(byte >> 4));
        text.push(hex_digit(byte & 0x0F));
    }
    Ok(text)
}

pub(super) fn abstract_address(name: &str) -> io::Result<SocketAddr> {
    SocketAddr::from_abstract_name(name.as_bytes())
}

pub(super) fn connect(address: &SocketAddr) -> io::Result<UnixStream> {
    UnixStream::connect_addr(address)
}

pub(super) fn is_current_user_peer(stream: &UnixStream) -> io::Result<bool> {
    let mut credentials = UCred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = u32::try_from(size_of::<UCred>()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Linux peer credential size is invalid",
        )
    })?;
    // SAFETY: the Unix stream owns a valid Linux socket descriptor.
    // `credentials` and `length` are writable storage with the declared sizes.
    let result = unsafe {
        getsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if usize::try_from(length).ok() != Some(size_of::<UCred>()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Linux peer credential shape is invalid",
        ));
    }
    // SAFETY: `geteuid` has no parameters and returns this process's effective
    // UID without allocating or mutating process state.
    Ok(matches_effective_uid(credentials.uid, unsafe { geteuid() }))
}

pub(super) const fn matches_effective_uid(peer_uid: u32, host_uid: u32) -> bool {
    peer_uid == host_uid
}

fn hex_digit(value: u8) -> char {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    char::from(DIGITS[usize::from(value)])
}
