use std::io;

use anodrel_folder_access::{FolderEntries, FolderEntry, FolderEntryKind, MAX_FOLDER_ENTRIES};

use crate::raw::{
    Dword, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, GetFileInformationByHandleEx,
    RetainedDirectory,
};

const FILE_FULL_DIRECTORY_INFO: Dword = 14;
const FILE_FULL_DIRECTORY_RESTART_INFO: Dword = 15;
const ERROR_NO_MORE_FILES: i32 = 18;
const DIRECTORY_BUFFER_BYTES: usize = 32 * 1024;
const FILE_FULL_DIR_INFO_HEADER_BYTES: usize = 68;
const NEXT_ENTRY_OFFSET: usize = 0;
const FILE_ATTRIBUTES_OFFSET: usize = 56;
const FILE_NAME_LENGTH_OFFSET: usize = 60;
const FILE_NAME_OFFSET: usize = 68;

/// Reads at most one bounded direct-entry snapshot from the retained handle.
pub(super) fn read_entries(folder: &mut RetainedDirectory) -> io::Result<FolderEntries> {
    let mut entries = Vec::with_capacity(MAX_FOLDER_ENTRIES);
    let mut restart = true;
    loop {
        let mut buffer = vec![0_u8; DIRECTORY_BUFFER_BYTES];
        let information_class = if restart {
            FILE_FULL_DIRECTORY_RESTART_INFO
        } else {
            FILE_FULL_DIRECTORY_INFO
        };
        let success = unsafe {
            // SAFETY: folder retains a live synchronous directory handle and
            // buffer provides exactly DIRECTORY_BUFFER_BYTES writable bytes.
            GetFileInformationByHandleEx(
                folder.handle(),
                information_class,
                buffer.as_mut_ptr().cast(),
                buffer.len() as Dword,
            )
        };
        restart = false;
        if success == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES) {
                return FolderEntries::new(entries, true)
                    .map_err(|_| io::Error::other("Windows entries were invalid"));
            }
            return Err(error);
        }
        if let Some(snapshot) = append_buffer_entries(&buffer, &mut entries)? {
            return Ok(snapshot);
        }
    }
}

fn append_buffer_entries(
    buffer: &[u8],
    entries: &mut Vec<FolderEntry>,
) -> io::Result<Option<FolderEntries>> {
    let mut offset = 0;
    loop {
        let record = parse_record(buffer, offset)?;
        if !matches!(record.name.as_str(), "." | "..") {
            if entries.len() == MAX_FOLDER_ENTRIES {
                return FolderEntries::new(std::mem::take(entries), false)
                    .map(Some)
                    .map_err(|_| io::Error::other("Windows entries were invalid"));
            }
            let kind = if record.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                FolderEntryKind::Other
            } else if record.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                FolderEntryKind::Directory
            } else {
                FolderEntryKind::File
            };
            let entry = FolderEntry::new(record.name, kind)
                .map_err(|_| io::Error::other("Windows entry name was invalid"))?;
            entries.push(entry);
        }
        let Some(next_offset) = record.next_offset else {
            return Ok(None);
        };
        offset = next_offset;
    }
}

struct DirectoryRecord {
    next_offset: Option<usize>,
    attributes: Dword,
    name: String,
}

fn parse_record(buffer: &[u8], offset: usize) -> io::Result<DirectoryRecord> {
    let header_end = offset
        .checked_add(FILE_FULL_DIR_INFO_HEADER_BYTES)
        .filter(|end| *end <= buffer.len())
        .ok_or_else(|| io::Error::other("Windows directory header was truncated"))?;
    let next = read_u32(buffer, offset + NEXT_ENTRY_OFFSET)? as usize;
    let record_end = if next == 0 {
        buffer.len()
    } else {
        let end = offset
            .checked_add(next)
            .filter(|end| *end <= buffer.len())
            .ok_or_else(|| io::Error::other("Windows directory record was truncated"))?;
        if next < FILE_FULL_DIR_INFO_HEADER_BYTES || !next.is_multiple_of(8) {
            return Err(io::Error::other("Windows directory record was invalid"));
        }
        end
    };
    let name_length = read_u32(buffer, offset + FILE_NAME_LENGTH_OFFSET)? as usize;
    if name_length == 0 || !name_length.is_multiple_of(2) {
        return Err(io::Error::other("Windows directory name was invalid"));
    }
    let name_start = offset + FILE_NAME_OFFSET;
    let name_end = name_start
        .checked_add(name_length)
        .filter(|end| *end <= record_end && *end <= buffer.len() && *end >= header_end)
        .ok_or_else(|| io::Error::other("Windows directory name was truncated"))?;
    let units = buffer[name_start..name_end]
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    let name = String::from_utf16(&units)
        .map_err(|_| io::Error::other("Windows directory name was not UTF-16"))?;
    Ok(DirectoryRecord {
        next_offset: (next != 0).then_some(record_end),
        attributes: read_u32(buffer, offset + FILE_ATTRIBUTES_OFFSET)?,
        name,
    })
}

fn read_u32(buffer: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| io::Error::other("Windows directory value was truncated"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::{FILE_FULL_DIR_INFO_HEADER_BYTES, parse_record};

    #[test]
    fn rejects_short_directory_records() {
        assert!(parse_record(&[0; FILE_FULL_DIR_INFO_HEADER_BYTES - 1], 0).is_err());
    }

    #[test]
    fn rejects_non_utf16_directory_names() {
        let mut bytes = vec![0; FILE_FULL_DIR_INFO_HEADER_BYTES + 1];
        bytes[60..64].copy_from_slice(&1_u32.to_le_bytes());
        assert!(parse_record(&bytes, 0).is_err());
    }
}
