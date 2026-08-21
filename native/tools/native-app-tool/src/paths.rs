//! Filesystem planning and relative dependency paths for generated projects.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::init::InitError;

/// Resolves a not-yet-created project below an existing directory.
pub fn resolve_new_project(destination: &Path) -> Result<PathBuf, InitError> {
    if destination.exists() {
        return Err(InitError::new("project destination already exists"));
    }
    let name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| InitError::new("project destination must name a directory"))?;
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|_| InitError::new("project destination parent is unavailable"))?;
    if !parent.is_dir() {
        return Err(InitError::new(
            "project destination parent is not a directory",
        ));
    }
    Ok(parent.join(name))
}

/// Locates the checkout that supplied this compiled tool.
pub fn anodrel_root() -> Result<PathBuf, InitError> {
    let tool_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = tool_directory
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| InitError::new("could not locate the Anodrel checkout"))?;
    root.canonicalize()
        .map_err(|_| InitError::new("could not locate the Anodrel checkout"))
}

/// Returns a lexical relative path between canonical paths on one filesystem.
pub fn relative_path(from: &Path, to: &Path) -> Result<PathBuf, InitError> {
    if from.is_absolute() != to.is_absolute() {
        return Err(InitError::new("could not calculate relative project paths"));
    }
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for component in &from[shared..] {
        match component {
            Component::Normal(_) => result.push(".."),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(InitError::new("could not calculate relative project paths"));
            }
        }
    }
    for component in &to[shared..] {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(InitError::new("could not calculate relative project paths"));
            }
        }
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Ok(result)
}

pub fn write_new_file(path: &Path, contents: &str) -> Result<(), InitError> {
    fs::write(path, contents).map_err(|_| InitError::new("could not write generated project files"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::relative_path;

    #[test]
    fn calculates_a_relative_path_without_an_absolute_checkout_path() {
        let path = relative_path(
            Path::new("workspace/generated/app"),
            Path::new("workspace/native/crates/client"),
        )
        .expect("paths share a root");
        assert_eq!(path, Path::new("../../native/crates/client"));
        assert!(!path.is_absolute());
    }

    #[test]
    fn rejects_paths_from_different_volumes() {
        assert!(
            relative_path(
                Path::new(r"C:\\workspace\\app"),
                Path::new(r"D:\\workspace\\client"),
            )
            .is_err()
        );
    }
}
