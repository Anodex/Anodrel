//! Bounded normal-directory collection for bundle authoring.

use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

use anodrel_release_bundle::{MAX_BUNDLE_BYTES, MAX_BUNDLE_ENTRIES, MAX_BUNDLE_PATH_BYTES};

use crate::BundleAuthorError;

const BUNDLE_HEADER_BYTES: usize = 8;
const BUNDLE_ENTRY_HEADER_BYTES: usize = 38;

/// One source file held until the deterministic encoder consumes it.
pub(super) struct SourceEntry {
    pub(super) path: String,
    pub(super) contents: Vec<u8>,
}

/// Reads one normal source tree into canonical bundle entry order.
pub(super) fn read_source_tree(
    source: &Path,
    output: &Path,
) -> Result<Vec<SourceEntry>, BundleAuthorError> {
    validate_source_and_output(source, output)?;
    let mut entries = Vec::new();
    collect_directory(source, source, &mut entries)?;
    entries.sort_unstable_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    Ok(entries)
}

fn validate_source_and_output(source: &Path, output: &Path) -> Result<(), BundleAuthorError> {
    if !source.is_absolute() || !is_normal_directory(source) {
        return Err(BundleAuthorError::SourceInvalid);
    }
    if !output.is_absolute() {
        return Err(BundleAuthorError::OutputInvalid);
    }
    if output
        .try_exists()
        .map_err(|_| BundleAuthorError::OutputInvalid)?
    {
        return Err(BundleAuthorError::OutputAlreadyExists);
    }
    let output_parent = output.parent().ok_or(BundleAuthorError::OutputInvalid)?;
    if !is_normal_directory(output_parent) {
        return Err(BundleAuthorError::OutputInvalid);
    }
    let source = fs::canonicalize(source).map_err(|_| BundleAuthorError::SourceInvalid)?;
    let output_parent =
        fs::canonicalize(output_parent).map_err(|_| BundleAuthorError::OutputInvalid)?;
    let output_name = output.file_name().ok_or(BundleAuthorError::OutputInvalid)?;
    let canonical_output = output_parent.join(output_name);
    if canonical_output.starts_with(source) {
        return Err(BundleAuthorError::OutputInvalid);
    }
    Ok(())
}

fn collect_directory(
    source: &Path,
    directory: &Path,
    entries: &mut Vec<SourceEntry>,
) -> Result<(), BundleAuthorError> {
    if !is_normal_directory(directory) {
        return Err(BundleAuthorError::SourceEntryInvalid);
    }
    let directory_entries =
        fs::read_dir(directory).map_err(|_| BundleAuthorError::SourceReadFailed)?;
    for entry in directory_entries {
        let entry = entry.map_err(|_| BundleAuthorError::SourceReadFailed)?;
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| BundleAuthorError::SourceReadFailed)?;
        if is_link_like(&metadata) {
            return Err(BundleAuthorError::SourceEntryInvalid);
        }
        if metadata.is_dir() {
            collect_directory(source, &path, entries)?;
        } else if metadata.is_file() {
            let path = relative_path(source, &path)?;
            entries.push(SourceEntry {
                contents: read_regular_file(&path, &entry.path(), entries)?,
                path,
            });
        } else {
            return Err(BundleAuthorError::SourceEntryInvalid);
        }
    }
    Ok(())
}

fn relative_path(source: &Path, entry: &Path) -> Result<String, BundleAuthorError> {
    let relative = entry
        .strip_prefix(source)
        .map_err(|_| BundleAuthorError::SourceEntryInvalid)?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(BundleAuthorError::SourceEntryInvalid);
        };
        parts.push(part.to_str().ok_or(BundleAuthorError::SourceEntryInvalid)?);
    }
    let path = parts.join("/");
    if path.is_empty() || path.len() > MAX_BUNDLE_PATH_BYTES {
        return Err(BundleAuthorError::SourceLimitExceeded);
    }
    Ok(path)
}

fn read_regular_file(
    bundle_path: &str,
    filesystem_path: &Path,
    existing_entries: &[SourceEntry],
) -> Result<Vec<u8>, BundleAuthorError> {
    if existing_entries.len() >= MAX_BUNDLE_ENTRIES {
        return Err(BundleAuthorError::SourceLimitExceeded);
    }
    let used = encoded_length(existing_entries)?;
    let entry_overhead = BUNDLE_ENTRY_HEADER_BYTES
        .checked_add(bundle_path.len())
        .ok_or(BundleAuthorError::SourceLimitExceeded)?;
    let available = MAX_BUNDLE_BYTES
        .checked_sub(used)
        .and_then(|remaining| remaining.checked_sub(entry_overhead))
        .ok_or(BundleAuthorError::SourceLimitExceeded)?;
    let file = fs::File::open(filesystem_path).map_err(|_| BundleAuthorError::SourceReadFailed)?;
    let metadata = file
        .metadata()
        .map_err(|_| BundleAuthorError::SourceReadFailed)?;
    if !metadata.is_file() || metadata.len() > available as u64 {
        return Err(BundleAuthorError::SourceLimitExceeded);
    }
    let mut contents = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(available));
    file.take((available as u64).saturating_add(1))
        .read_to_end(&mut contents)
        .map_err(|_| BundleAuthorError::SourceReadFailed)?;
    if contents.len() > available {
        return Err(BundleAuthorError::SourceLimitExceeded);
    }
    Ok(contents)
}

fn encoded_length(entries: &[SourceEntry]) -> Result<usize, BundleAuthorError> {
    entries
        .iter()
        .try_fold(BUNDLE_HEADER_BYTES, |length, entry| {
            length
                .checked_add(BUNDLE_ENTRY_HEADER_BYTES)
                .and_then(|length| length.checked_add(entry.path.len()))
                .and_then(|length| length.checked_add(entry.contents.len()))
                .ok_or(BundleAuthorError::SourceLimitExceeded)
        })
}

fn is_normal_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && !is_link_like(&metadata))
}

fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        metadata.file_attributes() & 0x0400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
pub(super) fn source_entries_for_test(
    source: &Path,
    output: &Path,
) -> Result<Vec<(String, Vec<u8>)>, BundleAuthorError> {
    read_source_tree(source, output).map(|entries| {
        entries
            .into_iter()
            .map(|entry| (entry.path, entry.contents))
            .collect()
    })
}
