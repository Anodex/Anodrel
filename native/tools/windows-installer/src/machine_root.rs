//! Fixed machine-owned application-root selection for the Windows installer.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use anodrel_application::is_valid_application_id;

/// One fixed machine-owned root selected from a signed application identity.
pub(crate) struct MachineApplicationRoot {
    path: PathBuf,
}

impl MachineApplicationRoot {
    /// Returns the private installer root for later staging and promotion composition.
    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl fmt::Debug for MachineApplicationRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MachineApplicationRoot")
            .field(
                "path_units",
                &self
                    .path()
                    .as_os_str()
                    .to_string_lossy()
                    .encode_utf16()
                    .count(),
            )
            .finish_non_exhaustive()
    }
}

/// The fixed 64-bit machine application root could not be established safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineRootError {
    /// Version 1 requires the 64-bit Program Files known folder.
    ArchitectureUnsupported,
    /// Windows could not initialize the shell call needed to select Program Files.
    KnownFolderUnavailable,
    /// A supplied signed application identity was invalid.
    ApplicationIdInvalid,
    /// A path contained a representation Windows cannot safely receive.
    PathInvalid,
    /// A fixed root component was missing, non-directory, or a reparse point.
    RootInvalid,
    /// A missing fixed root component could not be created.
    RootCreationFailed,
}

impl fmt::Display for MachineRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ArchitectureUnsupported => "this installer requires 64-bit Windows",
            Self::KnownFolderUnavailable => "Windows Program Files could not be located",
            Self::ApplicationIdInvalid => "the signed application identity is invalid",
            Self::PathInvalid => "the Windows installation path is invalid",
            Self::RootInvalid => "the fixed installation root is unsafe",
            Self::RootCreationFailed => "the fixed installation root could not be created",
        })
    }
}

impl std::error::Error for MachineRootError {}

/// Selects the one fixed 64-bit Program Files application root for an installer release.
///
/// The application identity must already be selected by the signed release
/// manifest. This accepts no caller-controlled base directory and performs no
/// staging, promotion, policy publication, launch, trust, or network work.
pub(crate) fn current_machine_application_root(
    application_id: &str,
) -> Result<MachineApplicationRoot, MachineRootError> {
    let program_files = raw::program_files_x64()?;
    application_root_below(&program_files, application_id)
}

/// Selects an existing fixed 64-bit Program Files application root without creating it.
pub(crate) fn existing_machine_application_root(
    application_id: &str,
) -> Result<MachineApplicationRoot, MachineRootError> {
    let program_files = raw::program_files_x64()?;
    existing_application_root_below(&program_files, application_id)
}

fn application_root_below(
    program_files: &Path,
    application_id: &str,
) -> Result<MachineApplicationRoot, MachineRootError> {
    if !is_valid_application_id(application_id) {
        return Err(MachineRootError::ApplicationIdInvalid);
    }
    raw::verify_normal_directory(program_files)?;
    let mut root = program_files.to_path_buf();
    for component in ["Anodrel", "Applications", application_id] {
        root.push(component);
        raw::create_or_verify_normal_directory(&root)?;
    }
    Ok(MachineApplicationRoot { path: root })
}

fn existing_application_root_below(
    program_files: &Path,
    application_id: &str,
) -> Result<MachineApplicationRoot, MachineRootError> {
    if !is_valid_application_id(application_id) {
        return Err(MachineRootError::ApplicationIdInvalid);
    }
    raw::verify_normal_directory(program_files)?;
    let mut root = program_files.to_path_buf();
    for component in ["Anodrel", "Applications", application_id] {
        root.push(component);
        raw::verify_normal_directory(&root)?;
    }
    let path = fs::canonicalize(root).map_err(|_| MachineRootError::RootInvalid)?;
    Ok(MachineApplicationRoot { path })
}

mod raw {
    use std::{
        ffi::{OsString, c_void},
        path::{Path, PathBuf},
        ptr,
    };

    use super::MachineRootError;

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    #[cfg(target_pointer_width = "64")]
    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn CreateDirectoryW(path_name: *const u16, attributes: *const c_void) -> i32;
        fn GetFileAttributesW(path_name: *const u16) -> u32;
        fn GetLastError() -> u32;
    }

    #[cfg(target_pointer_width = "64")]
    #[link(name = "Ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
        fn CoTaskMemFree(memory: *mut c_void);
        fn CoUninitialize();
    }
    #[cfg(target_pointer_width = "64")]
    #[link(name = "Shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(
            folder_id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }

    pub(super) fn verify_normal_directory(path: &Path) -> Result<(), MachineRootError> {
        let path = wide(path)?;
        // SAFETY: `path` remains NUL terminated for this read-only Windows call.
        let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
        (attributes != INVALID_FILE_ATTRIBUTES
            && attributes & FILE_ATTRIBUTE_DIRECTORY != 0
            && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0)
            .then_some(())
            .ok_or(MachineRootError::RootInvalid)
    }

    pub(super) fn create_or_verify_normal_directory(path: &Path) -> Result<(), MachineRootError> {
        let path_wide = wide(path)?;
        // SAFETY: `path_wide` is NUL terminated and no security attributes are requested.
        let created = unsafe { CreateDirectoryW(path_wide.as_ptr(), ptr::null()) };
        if created == 0 {
            // SAFETY: `GetLastError` obtains the status immediately after CreateDirectoryW.
            if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
                return Err(MachineRootError::RootCreationFailed);
            }
        }
        verify_normal_directory(path)
    }

    fn wide(path: &Path) -> Result<Vec<u16>, MachineRootError> {
        use std::os::windows::ffi::OsStrExt;

        let mut encoded = path.as_os_str().encode_wide().collect::<Vec<_>>();
        (!encoded.contains(&0))
            .then_some(())
            .ok_or(MachineRootError::PathInvalid)?;
        encoded.push(0);
        Ok(encoded)
    }

    #[cfg(target_pointer_width = "64")]
    pub(super) fn program_files_x64() -> Result<PathBuf, MachineRootError> {
        const COINIT_MULTITHREADED: u32 = 0;
        const MAX_PATH_UNITS: usize = 32_767;
        const PROGRAM_FILES_X64: Guid = Guid {
            data1: 0x6d80_9377,
            data2: 0x6af0,
            data3: 0x444b,
            data4: [0x89, 0x57, 0xa3, 0x77, 0x3f, 0x02, 0x20, 0x0e],
        };

        // SAFETY: no COM apartment is currently initialized by this short installer call.
        let initialized = unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED) };
        if initialized < 0 {
            return Err(MachineRootError::KnownFolderUnavailable);
        }
        let _com = ComGuard;
        let mut raw_path = ptr::null_mut();
        // SAFETY: all input pointers are valid and `raw_path` is one output slot.
        let status =
            unsafe { SHGetKnownFolderPath(&PROGRAM_FILES_X64, 0, ptr::null_mut(), &mut raw_path) };
        let path_guard = TaskMemoryGuard(raw_path);
        if status < 0 {
            return Err(MachineRootError::KnownFolderUnavailable);
        }
        path_from_wide(path_guard.0, MAX_PATH_UNITS).ok_or(MachineRootError::KnownFolderUnavailable)
    }

    #[cfg(target_pointer_width = "64")]
    struct ComGuard;

    #[cfg(target_pointer_width = "64")]
    impl Drop for ComGuard {
        fn drop(&mut self) {
            // SAFETY: the guard is created only after successful CoInitializeEx.
            unsafe { CoUninitialize() };
        }
    }

    #[cfg(target_pointer_width = "64")]
    struct TaskMemoryGuard(*mut u16);

    #[cfg(target_pointer_width = "64")]
    impl Drop for TaskMemoryGuard {
        fn drop(&mut self) {
            // SAFETY: Shell32 allocated this pointer for the matching CoTaskMemFree contract.
            unsafe { CoTaskMemFree(self.0.cast()) };
        }
    }

    #[cfg(target_pointer_width = "64")]
    fn path_from_wide(pointer: *const u16, max_units: usize) -> Option<PathBuf> {
        use std::os::windows::ffi::OsStringExt;

        if pointer.is_null() {
            return None;
        }
        let length = (0..=max_units).find(|index| {
            // SAFETY: Shell32 returned a NUL-terminated string; the strict bound limits reads.
            unsafe { *pointer.add(*index) == 0 }
        })?;
        // SAFETY: `length` was found at a NUL terminator within the bounded returned buffer.
        let units = unsafe { std::slice::from_raw_parts(pointer, length) };
        let path = PathBuf::from(OsString::from_wide(units));
        path.is_absolute().then_some(path)
    }

    #[cfg(not(target_pointer_width = "64"))]
    pub(super) fn program_files_x64() -> Result<PathBuf, MachineRootError> {
        Err(MachineRootError::ArchitectureUnsupported)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{MachineRootError, application_root_below, existing_application_root_below};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn builds_only_the_fixed_application_hierarchy_below_a_normal_machine_parent() {
        let parent = TemporaryDirectory::new();

        let root = application_root_below(parent.path(), "org.anodrel.root-test")
            .expect("the fixed hierarchy is created");
        let expected = parent
            .path()
            .join("Anodrel")
            .join("Applications")
            .join("org.anodrel.root-test");
        assert_eq!(root.path(), expected);
        assert!(expected.is_dir());
        assert_eq!(
            application_root_below(parent.path(), "org.anodrel.root-test")
                .expect("existing fixed components are retained")
                .path(),
            expected
        );
    }

    #[test]
    fn invalid_identity_cannot_create_a_machine_hierarchy() {
        let parent = TemporaryDirectory::new();
        assert!(matches!(
            application_root_below(parent.path(), "org.anodrel/escape"),
            Err(MachineRootError::ApplicationIdInvalid)
        ));
        assert!(!parent.path().join("Anodrel").exists());
    }

    #[test]
    fn existing_root_selection_never_creates_missing_components() {
        let parent = TemporaryDirectory::new();
        assert!(matches!(
            existing_application_root_below(parent.path(), "org.anodrel.root-test"),
            Err(MachineRootError::RootInvalid)
        ));
        let created = application_root_below(parent.path(), "org.anodrel.root-test")
            .expect("the fixed root is created for the fixture");
        assert_eq!(
            existing_application_root_below(parent.path(), "org.anodrel.root-test")
                .expect("the existing fixed root is selected")
                .path(),
            std::fs::canonicalize(created.path()).expect("the created fixed root canonicalizes")
        );
    }

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new() -> Self {
            for _ in 0..32 {
                let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "anodrel-machine-root-test-{}-{suffix}",
                    std::process::id()
                ));
                if std::fs::create_dir(&path).is_ok() {
                    return Self(path);
                }
            }
            panic!("a temporary machine-root test directory could not be created");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
