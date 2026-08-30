//! Strict host-session Wayland socket selection.

use std::{
    env,
    ffi::{CString, OsStr},
    os::unix::ffi::OsStrExt,
    path::{Component, Path},
};

const SUN_PATH_CAPACITY: usize = 108;

/// One validated local Wayland socket path whose text is never exposed.
pub(super) struct Locator {
    path: CString,
}

impl Locator {
    pub(super) fn from_environment() -> Result<Self, ()> {
        let runtime = env::var_os("XDG_RUNTIME_DIR").ok_or(())?;
        let display = env::var_os("WAYLAND_DISPLAY").ok_or(())?;
        Self::from_values(&runtime, &display)
    }

    fn from_values(runtime: &OsStr, display: &OsStr) -> Result<Self, ()> {
        let runtime = Path::new(runtime);
        if !runtime.is_absolute()
            || runtime.components().any(|component| {
                matches!(
                    component,
                    Component::CurDir | Component::ParentDir | Component::Prefix(_)
                )
            })
        {
            return Err(());
        }

        let display_path = Path::new(display);
        if !matches!(display_path.components().next(), Some(Component::Normal(_)))
            || display_path.components().nth(1).is_some()
        {
            return Err(());
        }

        let path = runtime.join(display_path);
        let bytes = path.as_os_str().as_bytes();
        if bytes.is_empty() || bytes.len() >= SUN_PATH_CAPACITY {
            return Err(());
        }
        CString::new(bytes)
            .map(|path| Self { path })
            .map_err(|_| ())
    }

    pub(super) fn path(&self) -> &std::ffi::CStr {
        &self.path
    }
}

impl std::fmt::Debug for Locator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Locator(..)")
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::Locator;

    #[test]
    fn accepts_one_display_name_under_an_absolute_runtime_directory() {
        assert!(
            Locator::from_values(OsStr::new("/run/user/1000"), OsStr::new("wayland-0")).is_ok()
        );
    }

    #[test]
    fn rejects_ambiguous_or_oversized_locator_values() {
        for display in ["", ".", "..", "/tmp/wayland-0", "nested/wayland-0"] {
            assert!(
                Locator::from_values(OsStr::new("/run/user/1000"), OsStr::new(display)).is_err()
            );
        }
        assert!(Locator::from_values(OsStr::new("relative"), OsStr::new("wayland-0")).is_err());
        assert!(
            Locator::from_values(OsStr::new("/run/user/1000"), OsStr::new(&"a".repeat(120)))
                .is_err()
        );
    }

    #[test]
    fn debug_does_not_disclose_the_desktop_socket_path() {
        let locator = Locator::from_values(OsStr::new("/run/user/1000"), OsStr::new("wayland-0"))
            .expect("fixture locator is valid");
        assert_eq!(format!("{locator:?}"), "Locator(..)");
    }
}
