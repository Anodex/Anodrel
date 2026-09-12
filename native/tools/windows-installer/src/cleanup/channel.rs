//! Deadline-bounded private pipe control frames; no target data crosses here.

use super::CleanupError;
use std::{
    io::Read,
    os::windows::io::AsRawHandle,
    thread,
    time::{Duration, Instant},
};

#[link(name = "Kernel32")]
unsafe extern "system" {
    fn PeekNamedPipe(
        pipe: *mut core::ffi::c_void,
        buffer: *mut core::ffi::c_void,
        size: u32,
        read: *mut u32,
        available: *mut u32,
        remaining: *mut u32,
    ) -> i32;
    fn GetStdHandle(kind: u32) -> *mut core::ffi::c_void;
    fn ReadFile(
        handle: *mut core::ffi::c_void,
        buffer: *mut u8,
        size: u32,
        read: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
    fn WriteFile(
        handle: *mut core::ffi::c_void,
        buffer: *const u8,
        size: u32,
        written: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
}

fn wait(handle: *mut core::ffi::c_void, timeout: Duration) -> Result<(), CleanupError> {
    let deadline = Instant::now() + timeout;
    loop {
        let mut available = 0;
        // SAFETY: Borrowed live pipe handle; only the available output is used.
        if unsafe {
            PeekNamedPipe(
                handle,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(CleanupError::Handoff);
        }
        if available > 4 {
            return Err(CleanupError::Handoff);
        }
        if available == 4 {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(CleanupError::Handoff);
        }
        thread::sleep(Duration::from_millis(10));
    }
}
pub(super) fn read(
    pipe: &mut (impl Read + AsRawHandle),
    expected: [u8; 4],
    timeout: Duration,
) -> Result<(), CleanupError> {
    wait(pipe.as_raw_handle(), timeout)?;
    let mut frame = [0; 4];
    pipe.read_exact(&mut frame)
        .map_err(|_| CleanupError::Handoff)?;
    (frame == expected)
        .then_some(())
        .ok_or(CleanupError::Handoff)
}
pub(super) fn read_stdin(expected: [u8; 4], timeout: Duration) -> Result<(), CleanupError> {
    // SAFETY: Borrow standard input without taking ownership of the handle.
    let handle = unsafe { GetStdHandle((-10_i32) as u32) };
    wait(handle, timeout)?;
    let mut frame = [0; 4];
    let mut count = 0;
    // SAFETY: Exactly four writable bytes, and PeekNamedPipe established availability.
    let ok = unsafe {
        ReadFile(
            handle,
            frame.as_mut_ptr(),
            4,
            &mut count,
            std::ptr::null_mut(),
        )
    };
    (ok != 0 && count == 4 && frame == expected)
        .then_some(())
        .ok_or(CleanupError::Handoff)
}
pub(super) fn write_stdout(frame: [u8; 4]) -> Result<(), CleanupError> {
    // SAFETY: Borrow the inherited output handle for one bounded write.
    let handle = unsafe { GetStdHandle((-11_i32) as u32) };
    let mut count = 0;
    // SAFETY: The frame remains alive through this synchronous four-byte write.
    let ok = unsafe { WriteFile(handle, frame.as_ptr(), 4, &mut count, std::ptr::null_mut()) };
    (ok != 0 && count == 4)
        .then_some(())
        .ok_or(CleanupError::Handoff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write, os::windows::io::FromRawHandle};
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn CreatePipe(
            read: *mut *mut core::ffi::c_void,
            write: *mut *mut core::ffi::c_void,
            security: *const core::ffi::c_void,
            size: u32,
        ) -> i32;
    }
    fn pipe() -> (File, File) {
        let mut r = std::ptr::null_mut();
        let mut w = std::ptr::null_mut();
        // SAFETY: Writable output handles; null security makes both non-inherited.
        assert_ne!(
            unsafe { CreatePipe(&mut r, &mut w, std::ptr::null(), 4096) },
            0
        );
        // SAFETY: The successful pipe call transfers two unique owned handles.
        unsafe { (File::from_raw_handle(r), File::from_raw_handle(w)) }
    }
    #[test]
    fn accepts_only_the_exact_ready_commit_and_accepted_frames() {
        for frame in [
            super::super::READY,
            super::super::COMMIT,
            super::super::ACCEPTED,
        ] {
            let (mut r, mut w) = pipe();
            w.write_all(&frame).unwrap();
            assert_eq!(read(&mut r, frame, Duration::from_millis(100)), Ok(()));
        }
        let (mut r, mut w) = pipe();
        w.write_all(b"BAD1").unwrap();
        assert_eq!(
            read(&mut r, super::super::COMMIT, Duration::from_millis(100)),
            Err(CleanupError::Handoff)
        );
    }
    #[test]
    fn truncated_silent_and_disconnected_channels_fail_within_deadline() {
        let (mut r, mut w) = pipe();
        assert_eq!(
            read(&mut r, super::super::COMMIT, Duration::from_millis(20)),
            Err(CleanupError::Handoff)
        );
        w.write_all(b"AC").unwrap();
        let start = Instant::now();
        assert_eq!(
            read(&mut r, super::super::COMMIT, Duration::from_millis(20)),
            Err(CleanupError::Handoff)
        );
        assert!(start.elapsed() < Duration::from_secs(1));
        drop(w);
        assert_eq!(
            read(&mut r, super::super::COMMIT, Duration::from_millis(20)),
            Err(CleanupError::Handoff)
        );
    }

    #[test]
    fn trailing_payload_cannot_extend_a_control_frame() {
        let (mut r, mut w) = pipe();
        w.write_all(b"ACC1unexpected target").unwrap();
        assert_eq!(
            read(&mut r, super::super::COMMIT, Duration::from_millis(20)),
            Err(CleanupError::Handoff)
        );
    }
}
