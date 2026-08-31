//! Direct same-volume Windows directory rename call.

use std::{os::windows::ffi::OsStrExt, path::Path};

use crate::PromotionError;

unsafe extern "system" {
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
}

/// Renames one existing stage to a sibling destination without copy or replacement.
pub(super) fn move_directory(source: &Path, destination: &Path) -> Result<(), PromotionError> {
    let source = wide_path(source);
    let destination = wide_path(destination);
    // SAFETY: Both paths are installer-derived absolute sibling directories.
    // Zero flags explicitly permit neither cross-volume copy nor replacement.
    let moved = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0) };
    (moved != 0)
        .then_some(())
        .ok_or(PromotionError::DirectoryMoveFailed)
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
