//! Direct normal-directory and exact private-image recovery operations.

use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
    ptr,
};

type Handle = isize;
type Bool = i32;
type Dword = u32;

const INVALID_HANDLE_VALUE: Handle = -1;
const FILE_ATTRIBUTE_DIRECTORY: Dword = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: Dword = 0x0000_0400;
const INVALID_FILE_ATTRIBUTES: Dword = u32::MAX;
const ERROR_ALREADY_EXISTS: Dword = 183;
const ERROR_FILE_NOT_FOUND: Dword = 2;
const ERROR_NO_MORE_FILES: Dword = 18;

#[repr(C)]
struct FindDataW {
    attributes: Dword,
    creation_time_low: Dword,
    creation_time_high: Dword,
    access_time_low: Dword,
    access_time_high: Dword,
    write_time_low: Dword,
    write_time_high: Dword,
    size_high: Dword,
    size_low: Dword,
    reserved_zero: Dword,
    reserved_one: Dword,
    name: [u16; 260],
    alternate_name: [u16; 14],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateDirectoryW(path: *const u16, security_attributes: *const core::ffi::c_void) -> Bool;
    fn DeleteFileW(path: *const u16) -> Bool;
    fn FindClose(find_file: Handle) -> Bool;
    fn FindFirstFileW(pattern: *const u16, data: *mut FindDataW) -> Handle;
    fn FindNextFileW(find_file: Handle, data: *mut FindDataW) -> Bool;
    fn GetFileAttributesW(path: *const u16) -> Dword;
    fn GetLastError() -> Dword;
}

/// Creates only a complete normal absolute directory tree.
pub(super) fn ensure_normal_directory_tree(path: &Path) -> Result<(), ()> {
    if !path.is_absolute() {
        return Err(());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::Normal(part) => {
                current.push(part);
                ensure_normal_directory(&current)?;
            }
            Component::CurDir | Component::ParentDir => return Err(()),
        }
    }
    Ok(())
}

/// Removes only normal files using the exact private update-image name grammar.
pub(super) fn recover_private_images(directory: &Path) -> Result<u32, ()> {
    let pattern = wide(&directory.join("*"));
    let mut data = std::mem::MaybeUninit::<FindDataW>::zeroed();
    // SAFETY: `pattern` is a null-terminated owned path and `data` points to a
    // writable FFI structure for Windows to initialize on success.
    let finder = unsafe { FindFirstFileW(pattern.as_ptr(), data.as_mut_ptr()) };
    if finder == INVALID_HANDLE_VALUE {
        return if last_error() == ERROR_FILE_NOT_FOUND {
            Ok(0)
        } else {
            Err(())
        };
    }
    // SAFETY: FindFirstFileW succeeded and initialized every field of `data`.
    let mut data = unsafe { data.assume_init() };
    let result = recover_found_images(finder, &mut data, directory);
    // SAFETY: this handle came from one successful FindFirstFileW call.
    unsafe {
        let _ = FindClose(finder);
    }
    result
}

fn recover_found_images(finder: Handle, data: &mut FindDataW, directory: &Path) -> Result<u32, ()> {
    let mut removed: u32 = 0;
    loop {
        if is_normal_file(data.attributes)
            && let Some(name) = file_name(data)
            && is_owned_update_image_name(&name)
            && delete_if_present(&directory.join(name))
        {
            removed = removed.checked_add(1).ok_or(())?;
        }
        // SAFETY: `finder` stays open until the caller closes it and `data` is
        // one writable FindDataW structure reused for the next record.
        if unsafe { FindNextFileW(finder, data) } == 0 {
            return if last_error() == ERROR_NO_MORE_FILES {
                Ok(removed)
            } else {
                Err(())
            };
        }
    }
}

fn ensure_normal_directory(path: &Path) -> Result<(), ()> {
    let wide_path = wide(path);
    // SAFETY: `wide_path` is an owned null-terminated path and no security
    // attributes are supplied.
    let created = unsafe { CreateDirectoryW(wide_path.as_ptr(), ptr::null()) };
    if created == 0 && last_error() != ERROR_ALREADY_EXISTS {
        return Err(());
    }
    normal_directory_attributes(path).then_some(()).ok_or(())
}

fn delete_if_present(path: &Path) -> bool {
    let wide_path = wide(path);
    // SAFETY: `wide_path` is an owned null-terminated file path selected only
    // from the fixed directory and exact private filename grammar.
    unsafe { DeleteFileW(wide_path.as_ptr()) != 0 }
}

fn normal_directory_attributes(path: &Path) -> bool {
    let attributes = attributes(path);
    attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
        == FILE_ATTRIBUTE_DIRECTORY
}

fn attributes(path: &Path) -> Dword {
    let wide_path = wide(path);
    // SAFETY: `wide_path` is an owned null-terminated Windows path.
    unsafe { GetFileAttributesW(wide_path.as_ptr()) }
}

fn is_normal_file(attributes: Dword) -> bool {
    attributes != INVALID_FILE_ATTRIBUTES
        && attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0
}

fn file_name(data: &FindDataW) -> Option<String> {
    let length = data.name.iter().position(|character| *character == 0)?;
    String::from_utf16(&data.name[..length]).ok()
}

fn is_owned_update_image_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix(".anodrel-update-")
        .and_then(|value| value.strip_suffix(".exe"))
    else {
        return false;
    };
    let Some((process, sequence)) = body.split_once('-') else {
        return false;
    };
    is_decimal_component(process, 10) && is_decimal_component(sequence, 20)
}

fn is_decimal_component(value: &str, maximum_length: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_length
        && value.bytes().all(|character| character.is_ascii_digit())
}

fn wide(path: &Path) -> Vec<u16> {
    OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn last_error() -> Dword {
    // SAFETY: GetLastError reads the immediately preceding current-thread FFI
    // result at each call site.
    unsafe { GetLastError() }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{ensure_normal_directory_tree, is_owned_update_image_name, recover_private_images};

    #[test]
    fn recovery_name_grammar_is_closed_and_bounded() {
        for accepted in [
            ".anodrel-update-1-1.exe",
            ".anodrel-update-4294967295-18446744073709551615.exe",
        ] {
            assert!(is_owned_update_image_name(accepted));
        }
        for rejected in [
            "anodrel-update-1-1.exe",
            ".anodrel-update--1.exe",
            ".anodrel-update-1-1.dll",
            ".anodrel-update-1-1-2.exe",
            ".anodrel-update-11111111111-1.exe",
            ".anodrel-update-1-111111111111111111111.exe",
        ] {
            assert!(!is_owned_update_image_name(rejected));
        }
    }

    #[test]
    fn recovery_removes_only_normal_exact_private_images() {
        let root = TemporaryDirectory::new();
        ensure_normal_directory_tree(root.path()).expect("fixture root is normal");
        let owned = root.path().join(".anodrel-update-7-9.exe");
        let unrelated = root.path().join("notes.txt");
        std::fs::write(&owned, b"owned image").expect("fixture image is written");
        std::fs::write(&unrelated, b"leave this file").expect("fixture file is written");

        assert_eq!(recover_private_images(root.path()), Ok(1));
        assert!(!owned.exists());
        assert!(unrelated.exists());
    }

    static NEXT_TEMPORARY_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!(
                "anodrel-update-cache-test-{}-{sequence}",
                std::process::id()
            )))
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
