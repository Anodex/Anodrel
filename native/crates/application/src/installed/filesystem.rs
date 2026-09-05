//! Filesystem facts for a host-selected installed application package.

use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use crate::{MAX_EXECUTABLE_BYTES, sha256};

use super::InstalledApplicationError;

pub(super) fn canonical_directory(path: &Path) -> std::io::Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    if fs::metadata(&path)?.is_dir() {
        Ok(path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path is not a directory",
        ))
    }
}

pub(crate) fn canonical_executable_path(
    package_root: &Path,
    declared_path: &str,
) -> Result<PathBuf, InstalledApplicationError> {
    let path = fs::canonicalize(package_root.join(declared_path))
        .map_err(InstalledApplicationError::Io)?;
    if !path.starts_with(package_root) {
        return Err(InstalledApplicationError::ExecutableOutsidePackage);
    }
    if !fs::metadata(&path)
        .map_err(InstalledApplicationError::Io)?
        .is_file()
    {
        return Err(InstalledApplicationError::ExecutableNotFile);
    }
    Ok(path)
}

pub(super) fn digest_file(path: &Path) -> Result<([u8; 32], usize), InstalledApplicationError> {
    let mut file = File::open(path).map_err(InstalledApplicationError::Io)?;
    sha256::digest_reader_limited(&mut file, MAX_EXECUTABLE_BYTES)
        .map_err(InstalledApplicationError::Io)?
        .ok_or(InstalledApplicationError::ExecutableTooLarge)
}

pub(super) fn read_limited(
    path: &Path,
    maximum: usize,
) -> Result<String, InstalledApplicationError> {
    let file = File::open(path).map_err(InstalledApplicationError::Io)?;
    let mut reader = file.take((maximum + 1) as u64);
    let mut contents = Vec::with_capacity(maximum.min(4_096));
    reader
        .read_to_end(&mut contents)
        .map_err(InstalledApplicationError::Io)?;
    if contents.len() > maximum {
        Err(InstalledApplicationError::RecordTooLarge)
    } else {
        String::from_utf8(contents).map_err(|_| InstalledApplicationError::InvalidRecord)
    }
}
