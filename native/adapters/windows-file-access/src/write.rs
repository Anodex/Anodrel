use std::{io, os::windows::ffi::OsStrExt, path::Path, ptr};

use crate::FileIdentity;

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const GENERIC_WRITE: Dword = 0x4000_0000;
const DELETE: Dword = 0x0001_0000;
const FILE_SHARE_READ: Dword = 0x0000_0001;
const CREATE_NEW: Dword = 1;
const OPEN_EXISTING: Dword = 3;
const FILE_ATTRIBUTE_NORMAL: Dword = 0x0000_0080;
const FILE_FLAG_OPEN_REPARSE_POINT: Dword = 0x0020_0000;
const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const FILE_DISPOSITION_INFO: Dword = 4;
const FILE_BEGIN: Dword = 0;

#[repr(C)]
struct ByHandleFileInformation {
    attributes: Dword,
    creation_time_low: Dword,
    creation_time_high: Dword,
    access_time_low: Dword,
    access_time_high: Dword,
    write_time_low: Dword,
    write_time_high: Dword,
    volume_serial: Dword,
    file_size_high: Dword,
    file_size_low: Dword,
    number_of_links: Dword,
    file_index_high: Dword,
    file_index_low: Dword,
}

#[repr(C)]
struct FileDispositionInformation {
    delete_file: Bool,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        desired_access: Dword,
        share_mode: Dword,
        security_attributes: *const core::ffi::c_void,
        creation_disposition: Dword,
        flags_and_attributes: Dword,
        template_file: Handle,
    ) -> Handle;
    fn GetFileInformationByHandle(file: Handle, information: *mut ByHandleFileInformation) -> Bool;
    fn SetFileInformationByHandle(
        file: Handle,
        information_class: Dword,
        information: *mut core::ffi::c_void,
        information_size: Dword,
    ) -> Bool;
    fn SetFilePointerEx(
        file: Handle,
        distance_to_move: i64,
        new_file_pointer: *mut i64,
        move_method: Dword,
    ) -> Bool;
    fn WriteFile(
        file: Handle,
        buffer: *const core::ffi::c_void,
        bytes_to_write: Dword,
        bytes_written: *mut Dword,
        overlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn SetEndOfFile(file: Handle) -> Bool;
    fn FlushFileBuffers(file: Handle) -> Bool;
    fn CloseHandle(handle: Handle) -> Bool;
}

/// One host-retained Windows output object.
///
/// The handle is never exposed beyond the native adapter. A newly created
/// target remains marked for deletion until a write begins, so abandoning its
/// opaque reference cannot leave an empty destination behind.
pub(super) struct WriteOnlyFile {
    handle: Handle,
    identity: FileIdentity,
    deletion_pending: bool,
}

impl WriteOnlyFile {
    pub(super) const fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub(super) fn write_text(&mut self, text: &str) -> io::Result<()> {
        if self.deletion_pending {
            set_deletion_pending(self.handle, false)?;
            self.deletion_pending = false;
        }
        let seeked = unsafe {
            // SAFETY: handle remains owned by this object, the operation is
            // synchronous, and the null result pointer intentionally discards
            // the file position after moving it to the documented origin.
            SetFilePointerEx(self.handle, 0, ptr::null_mut(), FILE_BEGIN)
        };
        if seeked == 0 {
            return Err(io::Error::last_os_error());
        }

        let mut remaining = text.as_bytes();
        while !remaining.is_empty() {
            let requested = remaining.len().min(Dword::MAX as usize) as Dword;
            let mut written = 0_u32;
            let completed = unsafe {
                // SAFETY: the retained synchronous handle is live; remaining
                // is valid for requested bytes until this call returns; no
                // OVERLAPPED state is used.
                WriteFile(
                    self.handle,
                    remaining.as_ptr().cast(),
                    requested,
                    &mut written,
                    ptr::null_mut(),
                )
            };
            if completed == 0 || written == 0 || written > requested {
                return Err(io::Error::last_os_error());
            }
            remaining = &remaining[written as usize..];
        }

        let truncated = unsafe {
            // SAFETY: the live synchronous file pointer now equals the number
            // of complete bytes written above, so SetEndOfFile removes only an
            // old tail from this retained object.
            SetEndOfFile(self.handle)
        };
        if truncated == 0 {
            return Err(io::Error::last_os_error());
        }
        let flushed = unsafe {
            // SAFETY: the live handle is a regular file opened for write. This
            // synchronous call has no caller-owned buffer or completion state.
            FlushFileBuffers(self.handle)
        };
        if flushed == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

pub(super) fn open_save_file(path: &Path) -> io::Result<WriteOnlyFile> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path is not absolute",
        ));
    }
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let existing = open(
        &wide,
        GENERIC_WRITE,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
    );
    if existing != INVALID_HANDLE_VALUE {
        return capture_existing(existing);
    }
    let existing_error = io::Error::last_os_error();
    if existing_error.raw_os_error() != Some(ERROR_FILE_NOT_FOUND) {
        return Err(existing_error);
    }

    let created = open(
        &wide,
        GENERIC_WRITE | DELETE,
        CREATE_NEW,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
    );
    if created == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    if let Err(error) = set_deletion_pending(created, true) {
        close(created);
        return Err(error);
    }
    match capture_regular_file(created, true) {
        Ok(file) => Ok(file),
        Err(error) => {
            close(created);
            Err(error)
        }
    }
}

fn open(
    wide: &[u16],
    desired_access: Dword,
    creation_disposition: Dword,
    flags_and_attributes: Dword,
) -> Handle {
    unsafe {
        // SAFETY: wide is NUL-terminated UTF-16. The synchronous operation
        // permits the null security pointer and no template handle. Readers
        // may coexist, but writers, rename, and delete are excluded while this
        // retained output handle is live.
        CreateFileW(
            wide.as_ptr(),
            desired_access,
            FILE_SHARE_READ,
            ptr::null(),
            creation_disposition,
            flags_and_attributes,
            0,
        )
    }
}

fn capture_existing(handle: Handle) -> io::Result<WriteOnlyFile> {
    match capture_regular_file(handle, false) {
        Ok(file) => Ok(file),
        Err(error) => {
            close(handle);
            Err(error)
        }
    }
}

fn capture_regular_file(handle: Handle, deletion_pending: bool) -> io::Result<WriteOnlyFile> {
    let mut information = std::mem::MaybeUninit::<ByHandleFileInformation>::uninit();
    let captured = unsafe {
        // SAFETY: handle is valid from successful CreateFileW and information
        // is writable storage for the exact Windows structure.
        GetFileInformationByHandle(handle, information.as_mut_ptr()) != 0
    };
    if !captured {
        return Err(io::Error::last_os_error());
    }
    let information = unsafe {
        // SAFETY: GetFileInformationByHandle reported successful initialization.
        information.assume_init()
    };
    if information.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected object is not a regular file",
        ));
    }
    let file_index =
        (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low);
    Ok(WriteOnlyFile {
        handle,
        identity: FileIdentity::new(information.volume_serial, file_index),
        deletion_pending,
    })
}

fn set_deletion_pending(handle: Handle, delete_file: bool) -> io::Result<()> {
    let mut information = FileDispositionInformation {
        delete_file: i32::from(delete_file),
    };
    let changed = unsafe {
        // SAFETY: handle is live and was opened with DELETE access when this
        // helper is used. information has the exact FileDispositionInfo shape.
        SetFileInformationByHandle(
            handle,
            FILE_DISPOSITION_INFO,
            (&mut information as *mut FileDispositionInformation).cast(),
            std::mem::size_of::<FileDispositionInformation>() as Dword,
        )
    };
    if changed == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl Drop for WriteOnlyFile {
    fn drop(&mut self) {
        close(self.handle);
    }
}

fn close(handle: Handle) {
    unsafe {
        // SAFETY: every call owns exactly one successful CreateFileW handle.
        let _ = CloseHandle(handle);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::open_save_file;

    #[test]
    fn deletion_information_uses_the_windows_bool_layout() {
        assert_eq!(
            std::mem::size_of::<super::FileDispositionInformation>(),
            std::mem::size_of::<super::Bool>()
        );
    }

    fn temporary_path(stem: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "anodrel-save-{stem}-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is valid")
                .as_nanos(),
        ))
    }

    #[test]
    fn capture_leaves_existing_contents_intact_until_write() {
        let path = temporary_path("existing");
        std::fs::write(&path, "before").expect("fixture is written");
        let file = open_save_file(&path).expect("existing fixture is captured");
        assert_ne!(file.identity().file_index(), 0);
        assert_eq!(
            std::fs::read_to_string(&path).expect("reader is allowed"),
            "before"
        );
        drop(file);
        assert_eq!(
            std::fs::read_to_string(&path).expect("fixture remains"),
            "before"
        );
        std::fs::remove_file(&path).expect("fixture is removed");
    }

    #[test]
    fn unused_new_capture_is_removed_when_its_handle_drops() {
        let path = temporary_path("unused");
        assert!(!path.exists());
        let file = open_save_file(&path).expect("new fixture is captured");
        assert!(path.exists());
        drop(file);
        assert!(!path.exists());
    }

    #[test]
    fn writes_complete_text_and_removes_an_old_tail() {
        let path = temporary_path("write");
        std::fs::write(&path, "a longer old value").expect("fixture is written");
        let mut file = open_save_file(&path).expect("fixture is captured");
        file.write_text("new").expect("write succeeds");
        drop(file);
        assert_eq!(
            std::fs::read_to_string(&path).expect("fixture is readable"),
            "new"
        );
        std::fs::remove_file(&path).expect("fixture is removed");
    }
}
