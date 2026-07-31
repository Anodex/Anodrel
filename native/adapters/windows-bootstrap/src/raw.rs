//! Narrow direct Kernel32 bindings for anonymous bootstrap pipe inheritance.

#![allow(non_snake_case)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::{io, mem, ptr};

pub type HandleValue = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: HandleValue = -1;
const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
const HANDLE_FLAG_INHERIT: Dword = 0x0000_0001;
const STARTF_USESTDHANDLES: Dword = 0x0000_0100;
const EXTENDED_STARTUPINFO_PRESENT: Dword = 0x0008_0000;
const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x0002_0002;
const GENERIC_WRITE: Dword = 0x4000_0000;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const FILE_SHARE_WRITE: Dword = 0x0000_0002;
const OPEN_EXISTING: Dword = 3;
const WAIT_OBJECT_0: Dword = 0;
const WAIT_TIMEOUT: Dword = 258;

#[repr(C)]
struct SecurityAttributes {
    length: Dword,
    security_descriptor: *mut core::ffi::c_void,
    inherit_handle: Bool,
}

#[repr(C)]
struct StartupInfoW {
    cb: Dword,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: Dword,
    y: Dword,
    x_size: Dword,
    y_size: Dword,
    x_count_chars: Dword,
    y_count_chars: Dword,
    fill_attribute: Dword,
    flags: Dword,
    show_window: u16,
    reserved2_count: u16,
    reserved2: *mut u8,
    standard_input: HandleValue,
    standard_output: HandleValue,
    standard_error: HandleValue,
}

#[repr(C)]
struct StartupInfoExW {
    startup_info: StartupInfoW,
    attribute_list: *mut core::ffi::c_void,
}

#[repr(C)]
struct ProcessInformation {
    process: HandleValue,
    thread: HandleValue,
    process_id: Dword,
    thread_id: Dword,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreatePipe(
        read_pipe: *mut HandleValue,
        write_pipe: *mut HandleValue,
        security_attributes: *const SecurityAttributes,
        size: Dword,
    ) -> Bool;
    fn SetHandleInformation(handle: HandleValue, mask: Dword, flags: Dword) -> Bool;
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const SecurityAttributes,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: HandleValue,
    ) -> HandleValue;
    fn InitializeProcThreadAttributeList(
        attribute_list: *mut core::ffi::c_void,
        attribute_count: Dword,
        flags: Dword,
        size: *mut usize,
    ) -> Bool;
    fn UpdateProcThreadAttribute(
        attribute_list: *mut core::ffi::c_void,
        flags: Dword,
        attribute: usize,
        value: *mut core::ffi::c_void,
        size: usize,
        previous_value: *mut core::ffi::c_void,
        return_size: *mut usize,
    ) -> Bool;
    fn DeleteProcThreadAttributeList(attribute_list: *mut core::ffi::c_void);
    fn CreateProcessW(
        application_name: *const u16,
        command_line: *mut u16,
        process_attributes: *const SecurityAttributes,
        thread_attributes: *const SecurityAttributes,
        inherit_handles: Bool,
        creation_flags: Dword,
        environment: *const core::ffi::c_void,
        current_directory: *const u16,
        startup_info: *mut StartupInfoW,
        process_information: *mut ProcessInformation,
    ) -> Bool;
    fn WriteFile(
        file: HandleValue,
        buffer: *const core::ffi::c_void,
        bytes_to_write: Dword,
        bytes_written: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn WaitForSingleObject(handle: HandleValue, milliseconds: Dword) -> Dword;
    fn GetExitCodeProcess(process: HandleValue, exit_code: *mut Dword) -> Bool;
    fn TerminateProcess(process: HandleValue, exit_code: Dword) -> Bool;
    fn CloseHandle(handle: HandleValue) -> Bool;
}

pub struct OwnedHandle(HandleValue);

impl OwnedHandle {
    fn new(handle: HandleValue) -> io::Result<Self> {
        if handle == 0 || handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }

    const fn value(&self) -> HandleValue {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: this RAII wrapper owns exactly one valid Kernel32 handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn launch_with_bootstrap(
    program: &str,
    command_line: &str,
    bootstrap: &[u8],
) -> io::Result<OwnedHandle> {
    let (child_input, parent_output) = create_bootstrap_pipe()?;
    let null_output = open_inheritable_null()?;
    let null_error = open_inheritable_null()?;
    let inherited = [child_input.value(), null_output.value(), null_error.value()];
    let attributes = AttributeList::new(&inherited)?;
    let process = create_process(
        program,
        command_line,
        child_input.value(),
        null_output.value(),
        null_error.value(),
        &attributes,
    )?;
    drop(attributes);
    drop(child_input);
    drop(null_output);
    drop(null_error);
    if let Err(error) = write_all(&parent_output, bootstrap) {
        terminate_after_delivery_failure(&process);
        return Err(error);
    }
    drop(parent_output);
    Ok(process)
}

fn terminate_after_delivery_failure(process: &OwnedHandle) {
    // SAFETY: process is a valid child process handle. A launch that cannot
    // deliver its sole credential must not leave an unbootstrapped child alive.
    unsafe {
        TerminateProcess(process.value(), 1);
    }
}

pub fn wait_for_exit(process: &OwnedHandle, timeout_milliseconds: u32) -> io::Result<u32> {
    // SAFETY: process is a valid process handle, owned for this call.
    match unsafe { WaitForSingleObject(process.value(), timeout_milliseconds) } {
        WAIT_OBJECT_0 => {
            let mut exit_code = 0;
            // SAFETY: exit_code is writable and process has signalled.
            if unsafe { GetExitCodeProcess(process.value(), &mut exit_code) } == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(exit_code)
            }
        }
        WAIT_TIMEOUT => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "bootstrap child timed out",
        )),
        _ => Err(io::Error::last_os_error()),
    }
}

fn create_bootstrap_pipe() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let attributes = SecurityAttributes {
        length: mem::size_of::<SecurityAttributes>() as Dword,
        security_descriptor: ptr::null_mut(),
        inherit_handle: 1,
    };
    let mut child_input = 0;
    let mut parent_output = 0;
    // SAFETY: the outputs point to writable handle storage and attributes is
    // initialized for an inheritable anonymous pipe pair.
    if unsafe { CreatePipe(&mut child_input, &mut parent_output, &attributes, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let child_input = OwnedHandle::new(child_input)?;
    let parent_output = OwnedHandle::new(parent_output)?;
    // SAFETY: parent_output is valid and this clears only its inheritance bit.
    if unsafe { SetHandleInformation(parent_output.value(), HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((child_input, parent_output))
}

fn open_inheritable_null() -> io::Result<OwnedHandle> {
    let attributes = SecurityAttributes {
        length: mem::size_of::<SecurityAttributes>() as Dword,
        security_descriptor: ptr::null_mut(),
        inherit_handle: 1,
    };
    let nul = wide_null("NUL");
    // SAFETY: nul is a null-terminated device name and attributes remains
    // valid for this synchronous open call.
    let handle = unsafe {
        CreateFileW(
            nul.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &attributes,
            OPEN_EXISTING,
            0,
            0,
        )
    };
    OwnedHandle::new(handle)
}

fn create_process(
    program: &str,
    command_line: &str,
    standard_input: HandleValue,
    standard_output: HandleValue,
    standard_error: HandleValue,
    attributes: &AttributeList,
) -> io::Result<OwnedHandle> {
    let program = wide_null(program);
    let mut command_line = wide_null(command_line);
    let mut startup = StartupInfoExW {
        startup_info: StartupInfoW {
            cb: mem::size_of::<StartupInfoExW>() as Dword,
            reserved: ptr::null_mut(),
            desktop: ptr::null_mut(),
            title: ptr::null_mut(),
            x: 0,
            y: 0,
            x_size: 0,
            y_size: 0,
            x_count_chars: 0,
            y_count_chars: 0,
            fill_attribute: 0,
            flags: STARTF_USESTDHANDLES,
            show_window: 0,
            reserved2_count: 0,
            reserved2: ptr::null_mut(),
            standard_input,
            standard_output,
            standard_error,
        },
        attribute_list: attributes.pointer(),
    };
    let mut process = ProcessInformation {
        process: 0,
        thread: 0,
        process_id: 0,
        thread_id: 0,
    };
    // SAFETY: all strings are null-terminated UTF-16, the extended startup
    // structure contains a valid explicit inherited-handle list, and process
    // points to writable result storage. The environment is inherited but no
    // bootstrap material is placed in it.
    if unsafe {
        CreateProcessW(
            program.as_ptr(),
            command_line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT,
            ptr::null(),
            ptr::null(),
            &mut startup.startup_info,
            &mut process,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let process_handle = OwnedHandle::new(process.process)?;
    let thread_handle = OwnedHandle::new(process.thread)?;
    drop(thread_handle);
    Ok(process_handle)
}

fn write_all(handle: &OwnedHandle, input: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < input.len() {
        let remaining = &input[offset..];
        let count = u32::try_from(remaining.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "bootstrap frame is too large")
        })?;
        let mut written = 0;
        // SAFETY: remaining remains valid for this synchronous write and the
        // parent write endpoint is uniquely owned for the operation.
        if unsafe {
            WriteFile(
                handle.value(),
                remaining.as_ptr().cast(),
                count,
                &mut written,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "bootstrap pipe wrote zero bytes",
            ));
        }
        offset += written as usize;
    }
    Ok(())
}

struct AttributeList {
    storage: Vec<usize>,
}

impl AttributeList {
    fn new(handles: &[HandleValue]) -> io::Result<Self> {
        let mut required_bytes = 0;
        // SAFETY: the first call intentionally queries the required size.
        let initialized = unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut required_bytes)
        };
        if initialized != 0
            || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER)
        {
            return Err(io::Error::last_os_error());
        }
        let words = required_bytes.div_ceil(mem::size_of::<usize>());
        let mut storage = vec![0_usize; words];
        // SAFETY: storage is suitably aligned and has at least the size that
        // InitializeProcThreadAttributeList requested.
        if unsafe {
            InitializeProcThreadAttributeList(
                storage.as_mut_ptr().cast(),
                1,
                0,
                &mut required_bytes,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the initialized list is live, and handles is a non-empty
        // contiguous array that remains live until CreateProcessW returns.
        if unsafe {
            UpdateProcThreadAttribute(
                storage.as_mut_ptr().cast(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                handles.as_ptr().cast_mut().cast(),
                mem::size_of_val(handles),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } == 0
        {
            // SAFETY: initialization succeeded, so this list needs cleanup.
            unsafe { DeleteProcThreadAttributeList(storage.as_mut_ptr().cast()) };
            return Err(io::Error::last_os_error());
        }
        Ok(Self { storage })
    }

    fn pointer(&self) -> *mut core::ffi::c_void {
        self.storage.as_ptr().cast_mut().cast()
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        // SAFETY: construction succeeds only after this list is initialized.
        unsafe { DeleteProcThreadAttributeList(self.storage.as_mut_ptr().cast()) };
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
