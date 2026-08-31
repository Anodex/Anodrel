//! Version-1 binary release-bundle codec.

use anodrel_application::sha256;

use crate::{MAX_BUNDLE_BYTES, MAX_BUNDLE_ENTRIES, MAX_BUNDLE_PATH_BYTES, ReleaseBundleError};

const MAGIC: [u8; 4] = *b"ANDB";
const HEADER_BYTES: usize = 8;
const ENTRY_HEADER_BYTES: usize = 38;

/// One regular file supplied to the deterministic bundle encoder.
#[derive(Clone, Copy, Debug)]
pub struct BundleEntryInput<'data> {
    /// A strictly ordered relative UTF-8 file path.
    pub path: &'data str,
    /// Exact raw file bytes.
    pub contents: &'data [u8],
}

/// One checked regular file borrowed from a parsed bundle.
#[derive(Debug)]
pub struct BundleEntry<'bundle> {
    path: String,
    contents: &'bundle [u8],
}

impl BundleEntry<'_> {
    /// Returns the checked relative file path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the checked borrowed raw file bytes.
    #[must_use]
    pub fn contents(&self) -> &[u8] {
        self.contents
    }
}

/// A fully checked, borrowed release bundle.
#[derive(Debug)]
pub struct ReleaseBundle<'bundle> {
    entries: Vec<BundleEntry<'bundle>>,
}

impl<'bundle> ReleaseBundle<'bundle> {
    /// Parses a bounded version-1 bundle and checks every entry digest.
    pub fn parse(bytes: &'bundle [u8]) -> Result<Self, ReleaseBundleError> {
        if bytes.len() > MAX_BUNDLE_BYTES {
            return Err(ReleaseBundleError::TooLarge);
        }
        let mut cursor = Cursor::new(bytes);
        if cursor.take(4)? != MAGIC {
            return Err(ReleaseBundleError::HeaderInvalid);
        }
        match (cursor.byte()?, cursor.byte()?) {
            (1, 0) => {}
            _ => return Err(ReleaseBundleError::VersionUnsupported),
        }
        let count = usize::from(cursor.u16()?);
        if count > MAX_BUNDLE_ENTRIES {
            return Err(ReleaseBundleError::EntryCountInvalid);
        }

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let path_length = usize::from(cursor.u16()?);
            let content_length =
                usize::try_from(cursor.u32()?).map_err(|_| ReleaseBundleError::Truncated)?;
            let digest = cursor.take(32)?;
            let path_bytes = cursor.take(path_length)?;
            let path =
                std::str::from_utf8(path_bytes).map_err(|_| ReleaseBundleError::PathInvalid)?;
            if !is_valid_path(path) {
                return Err(ReleaseBundleError::PathInvalid);
            }
            if entries
                .last()
                .is_some_and(|entry: &BundleEntry<'_>| entry.path.as_str() >= path)
            {
                return Err(ReleaseBundleError::EntryOrderInvalid);
            }
            let contents = cursor.take(content_length)?;
            if sha256::digest(contents).as_slice() != digest {
                return Err(ReleaseBundleError::DigestMismatch);
            }
            entries.push(BundleEntry {
                path: path.to_owned(),
                contents,
            });
        }
        if !cursor.finished() {
            return Err(ReleaseBundleError::TrailingData);
        }
        Ok(Self { entries })
    }

    /// Returns all checked entries in their canonical bundle order.
    #[must_use]
    pub fn entries(&self) -> &[BundleEntry<'bundle>] {
        &self.entries
    }

    /// Returns one checked entry's bytes by its exact relative path.
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.entries
            .binary_search_by(|entry| entry.path().cmp(path))
            .ok()
            .map(|index| self.entries[index].contents())
    }
}

/// Encodes a strictly ordered collection of regular files as version-1 bytes.
///
/// The caller supplies paths in canonical order so equal release content always
/// produces exactly the same bytes. The encoder performs no filesystem I/O.
pub fn encode(entries: &[BundleEntryInput<'_>]) -> Result<Vec<u8>, ReleaseBundleError> {
    if entries.len() > MAX_BUNDLE_ENTRIES {
        return Err(ReleaseBundleError::EntryCountInvalid);
    }
    let mut output_length = HEADER_BYTES;
    let mut previous = None;
    for entry in entries {
        validate_input(entry, previous)?;
        previous = Some(entry.path);
        output_length = output_length
            .checked_add(ENTRY_HEADER_BYTES)
            .and_then(|length| length.checked_add(entry.path.len()))
            .and_then(|length| length.checked_add(entry.contents.len()))
            .ok_or(ReleaseBundleError::TooLarge)?;
        if output_length > MAX_BUNDLE_BYTES {
            return Err(ReleaseBundleError::TooLarge);
        }
    }

    let mut output = Vec::with_capacity(output_length);
    output.extend_from_slice(&MAGIC);
    output.extend_from_slice(&[1, 0]);
    output.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for entry in entries {
        output.extend_from_slice(&(entry.path.len() as u16).to_le_bytes());
        output.extend_from_slice(&(entry.contents.len() as u32).to_le_bytes());
        output.extend_from_slice(&sha256::digest(entry.contents));
        output.extend_from_slice(entry.path.as_bytes());
        output.extend_from_slice(entry.contents);
    }
    Ok(output)
}

fn validate_input(
    entry: &BundleEntryInput<'_>,
    previous: Option<&str>,
) -> Result<(), ReleaseBundleError> {
    if !is_valid_path(entry.path) {
        return Err(ReleaseBundleError::PathInvalid);
    }
    if previous.is_some_and(|prior| prior >= entry.path) {
        return Err(ReleaseBundleError::EntryOrderInvalid);
    }
    Ok(())
}

fn is_valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_BUNDLE_PATH_BYTES
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.chars().any(char::is_control)
        })
}

struct Cursor<'input> {
    input: &'input [u8],
    offset: usize,
}

impl<'input> Cursor<'input> {
    fn new(input: &'input [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn byte(&mut self) -> Result<u8, ReleaseBundleError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ReleaseBundleError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, ReleaseBundleError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn take(&mut self, length: usize) -> Result<&'input [u8], ReleaseBundleError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ReleaseBundleError::Truncated)?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(ReleaseBundleError::Truncated)?;
        self.offset = end;
        Ok(bytes)
    }

    fn finished(&self) -> bool {
        self.offset == self.input.len()
    }
}
