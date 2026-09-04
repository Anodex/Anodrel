//! Windows-safe path derivation below a private staging directory.

use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_WINDOWS_PATH_UNITS: usize = 260;

/// Derives one safe absolute output path from a canonical bundle path.
pub(super) fn output_path(root: &Path, bundle_path: &str) -> Option<PathBuf> {
    let mut output = root.to_path_buf();
    for component in bundle_path.split('/') {
        if !safe_component(component) {
            return None;
        }
        output.push(component);
    }
    (windows_path_units(&output) <= MAX_WINDOWS_PATH_UNITS).then_some(output)
}

/// Creates only missing normal directories already below the supplied stage root.
pub(crate) fn create_private_directories(root: &Path, target: &Path) -> Result<(), ()> {
    let relative = target.strip_prefix(root).map_err(|_| ())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(());
        };
        current.push(component);
        match fs::create_dir(&current) {
            Ok(()) => verify_normal_directory(&current)?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                verify_normal_directory(&current)?;
            }
            Err(_) => return Err(()),
        }
    }
    Ok(())
}

fn verify_normal_directory(path: &Path) -> Result<(), ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    (!metadata.file_type().is_symlink() && metadata.is_dir())
        .then_some(())
        .ok_or(())
}

fn safe_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && !component.ends_with(['.', ' '])
        && !component.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && !component.chars().any(char::is_control)
        && !reserved_device_name(component)
}

fn reserved_device_name(component: &str) -> bool {
    let base = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            base.as_bytes(),
            [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']
        )
        || matches!(
            base.as_str(),
            "COM¹" | "COM²" | "COM³" | "LPT¹" | "LPT²" | "LPT³"
        )
}

fn windows_path_units(path: &Path) -> usize {
    path.as_os_str().to_string_lossy().encode_utf16().count() + 1
}

#[cfg(test)]
mod tests {
    use super::output_path;

    #[test]
    fn windows_only_device_and_normalization_hazards_are_rejected() {
        let root = std::path::Path::new("C:\\Anodrel");
        for unsafe_path in [
            "bin/CON.exe",
            "bin/Aux.txt",
            "bin/lpt9.log",
            "bin/COM¹.txt",
            "bin/name.",
            "bin/name ",
            "bin/quo?te.txt",
        ] {
            assert!(
                output_path(root, unsafe_path).is_none(),
                "accepted {unsafe_path}"
            );
        }
        assert!(output_path(root, "bin/Product.exe").is_some());
    }
}
