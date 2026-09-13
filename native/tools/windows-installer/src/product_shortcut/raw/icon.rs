//! Atomic persistence of the one shared Anodrel Start-menu icon.

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use anodrel_brand::app_icon;

use super::{ShortcutWriteError, TemporaryFile, verify_absent_or_regular_file};

/// Writes the fixed host-owned icon under one verified Anodrel shell directory.
pub(super) fn replace(directory: &Path) -> Result<PathBuf, ShortcutWriteError> {
    let destination = directory.join(app_icon::WINDOWS_FILE_NAME);
    verify_absent_or_regular_file(&destination)?;
    let temporary = TemporaryFile::create(directory, "tmp.ico")?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(temporary.path())
        .map_err(|_| ShortcutWriteError::IconWriteFailed)?;
    file.write_all(&app_icon::windows_ico())
        .and_then(|()| file.sync_all())
        .map_err(|_| ShortcutWriteError::IconWriteFailed)?;
    drop(file);
    temporary
        .replace(&destination)
        .map_err(|_| ShortcutWriteError::IconWriteFailed)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::replace;
    use crate::test_support::TestDirectory;
    use anodrel_brand::app_icon;

    #[test]
    fn writes_the_fixed_brand_icon_as_an_ordinary_file() {
        let directory = TestDirectory::new("shortcut-icon");
        let icon = replace(directory.path()).expect("the fixed icon persists");
        assert_eq!(icon.file_name().unwrap(), "Anodrel.ico");
        let expected = app_icon::windows_ico();
        assert_eq!(std::fs::read(&icon).unwrap(), expected);

        let replacement = replace(directory.path()).expect("the icon replaces atomically");
        assert_eq!(replacement, icon);
        assert_eq!(std::fs::read(&icon).unwrap(), app_icon::windows_ico());
        let files = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(files, [std::ffi::OsString::from("Anodrel.ico")]);
    }
}
