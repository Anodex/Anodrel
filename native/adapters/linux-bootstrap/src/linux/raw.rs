//! Direct Linux process and descriptor calls for one invited development child.

use std::{
    ffi::{CString, c_char, c_int},
    io,
    os::fd::RawFd,
    thread,
    time::{Duration, Instant},
};

const O_WRONLY: c_int = 1;
const O_CLOEXEC: c_int = 0o2000000;
const STDIN: c_int = 0;
const STDOUT: c_int = 1;
const STDERR: c_int = 2;
const WNOHANG: c_int = 1;
const EINTR: i32 = 4;
const SIGTERM: c_int = 15;
const SIGKILL: c_int = 9;
const EXEC_FAILURE_EXIT: c_int = 127;
const WAIT_INTERVAL: Duration = Duration::from_millis(10);

#[link(name = "c")]
unsafe extern "C" {
    fn pipe2(descriptors: *mut c_int, flags: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn fork() -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(descriptor: c_int) -> c_int;
    fn execve(
        path: *const c_char,
        arguments: *const *const c_char,
        environment: *const *const c_char,
    ) -> c_int;
    fn _exit(status: c_int) -> !;
    fn write(descriptor: c_int, bytes: *const core::ffi::c_void, count: usize) -> isize;
    fn kill(process: c_int, signal: c_int) -> c_int;
    fn waitpid(process: c_int, status: *mut c_int, options: c_int) -> c_int;
}

/// An owned close-on-exec file descriptor.
struct OwnedDescriptor(RawFd);

impl OwnedDescriptor {
    fn new(descriptor: RawFd) -> Result<Self, ()> {
        if descriptor < 0 {
            Err(())
        } else {
            Ok(Self(descriptor))
        }
    }

    fn value(&self) -> RawFd {
        self.0
    }
}

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns exactly one descriptor returned by Linux.
        unsafe {
            let _ = close(self.0);
        }
    }
}

/// Starts one child and delivers its complete private bootstrap frame.
pub(super) fn launch(program: &CString, bootstrap: &[u8]) -> Result<c_int, ()> {
    let (child_input, parent_output) = bootstrap_pipe()?;
    let null_output = open_null_output()?;
    let arguments = [program.as_ptr(), std::ptr::null()];
    let environment = [std::ptr::null()];

    // SAFETY: the parent has prepared all C strings, descriptors, and pointer
    // arrays before fork. The child path below performs only descriptor,
    // execve, and _exit calls before replacing its image.
    let process = unsafe { fork() };
    if process < 0 {
        return Err(());
    }
    if process == 0 {
        child_exec(
            child_input.value(),
            parent_output.value(),
            null_output.value(),
            program.as_ptr(),
            arguments.as_ptr(),
            environment.as_ptr(),
        );
    }

    drop(child_input);
    drop(null_output);
    if write_all(parent_output.value(), bootstrap).is_err() {
        drop(parent_output);
        reap_after_failed_delivery(process);
        return Err(());
    }
    drop(parent_output);
    Ok(process)
}

/// Waits for a child exit within one host-selected bounded interval.
pub(super) fn wait_for_exit(process: c_int, timeout: Duration) -> Result<u32, WaitError> {
    let deadline = Instant::now() + timeout;
    loop {
        match poll_exit(process)? {
            Some(status) => return exit_code(status).ok_or(WaitError::Unavailable),
            None if Instant::now() >= deadline => return Err(WaitError::TimedOut),
            None => thread::sleep(WAIT_INTERVAL),
        }
    }
}

/// Sends the fixed host termination signal without choosing a child signal.
pub(super) fn terminate(process: c_int) -> Result<(), ()> {
    // SAFETY: process is a child PID supplied only by launch and SIGTERM is the
    // fixed documented host termination signal.
    if unsafe { kill(process, SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Closed wait categories for an opaque child.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WaitError {
    TimedOut,
    Unavailable,
}

fn bootstrap_pipe() -> Result<(OwnedDescriptor, OwnedDescriptor), ()> {
    let mut descriptors = [-1, -1];
    // SAFETY: descriptors is writable storage for exactly two Linux pipe ends.
    if unsafe { pipe2(descriptors.as_mut_ptr(), O_CLOEXEC) } != 0 {
        return Err(());
    }
    Ok((
        OwnedDescriptor::new(descriptors[0])?,
        OwnedDescriptor::new(descriptors[1])?,
    ))
}

fn open_null_output() -> Result<OwnedDescriptor, ()> {
    let path = c"/dev/null";
    // SAFETY: path is one static NUL-terminated device name and flags are fixed.
    let descriptor = unsafe { open(path.as_ptr(), O_WRONLY | O_CLOEXEC, 0) };
    OwnedDescriptor::new(descriptor)
}

fn child_exec(
    child_input: RawFd,
    parent_output: RawFd,
    null_output: RawFd,
    program: *const c_char,
    arguments: *const *const c_char,
    environment: *const *const c_char,
) -> ! {
    if redirect(child_input, STDIN).is_err()
        || redirect(null_output, STDOUT).is_err()
        || redirect(null_output, STDERR).is_err()
    {
        // SAFETY: _exit is the async-signal-safe child failure route.
        unsafe { _exit(EXEC_FAILURE_EXIT) };
    }
    close_non_standard(child_input);
    close_non_standard(parent_output);
    close_non_standard(null_output);
    // SAFETY: every pointer was built before fork from static or parent-owned
    // C storage. execve either replaces the process image or returns an error.
    unsafe {
        let _ = execve(program, arguments, environment);
        _exit(EXEC_FAILURE_EXIT);
    }
}

fn redirect(source: RawFd, target: RawFd) -> Result<(), ()> {
    if source == target {
        return Ok(());
    }
    // SAFETY: both descriptors are valid in the child immediately after fork.
    if unsafe { dup2(source, target) } < 0 {
        Err(())
    } else {
        Ok(())
    }
}

fn close_non_standard(descriptor: RawFd) {
    if descriptor > STDERR {
        // SAFETY: this closes one inherited descriptor after standard routing.
        unsafe {
            let _ = close(descriptor);
        }
    }
}

fn write_all(descriptor: RawFd, bytes: &[u8]) -> Result<(), ()> {
    let mut offset = 0;
    while offset < bytes.len() {
        // SAFETY: bytes remains valid for the synchronous bounded pipe write.
        let written = unsafe {
            write(
                descriptor,
                bytes[offset..].as_ptr().cast(),
                bytes.len() - offset,
            )
        };
        if written > 0 {
            offset += written as usize;
        } else if written < 0 && io::Error::last_os_error().raw_os_error() == Some(EINTR) {
            continue;
        } else {
            return Err(());
        }
    }
    Ok(())
}

fn reap_after_failed_delivery(process: c_int) {
    // SAFETY: process came from this launch and SIGKILL prevents an uninvited
    // child from surviving a parent bootstrap-delivery failure.
    unsafe {
        let _ = kill(process, SIGKILL);
    }
    loop {
        let mut status = 0;
        // SAFETY: status is writable, and waitpid reaps only this child.
        let waited = unsafe { waitpid(process, &mut status, 0) };
        if waited == process
            || waited < 0 && io::Error::last_os_error().raw_os_error() != Some(EINTR)
        {
            return;
        }
    }
}

fn poll_exit(process: c_int) -> Result<Option<c_int>, WaitError> {
    loop {
        let mut status = 0;
        // SAFETY: status is writable, and WNOHANG polls only this child.
        let waited = unsafe { waitpid(process, &mut status, WNOHANG) };
        if waited == process {
            return Ok(Some(status));
        }
        if waited == 0 {
            return Ok(None);
        }
        if io::Error::last_os_error().raw_os_error() == Some(EINTR) {
            continue;
        }
        return Err(WaitError::Unavailable);
    }
}

fn exit_code(status: c_int) -> Option<u32> {
    let signal = status & 0x7f;
    if signal == 0 {
        Some(((status >> 8) & 0xff) as u32)
    } else if signal != 0x7f {
        Some((128 + signal) as u32)
    } else {
        None
    }
}
