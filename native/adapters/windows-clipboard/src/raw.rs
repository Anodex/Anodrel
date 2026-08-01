//! Narrow User32 and Kernel32 bindings for bounded Unicode clipboard text.

use std::{ffi::c_void, mem, ptr, slice};

use anodrel_clipboard::MAX_CLIPBOARD_TEXT_BYTES;

type Bool = i32;
type Handle = isize;

const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x0002;
const MAX_WINDOWS_TEXT_BYTES: usize = MAX_CLIPBOARD_TEXT_BYTES * 2 + mem::size_of::<u16>();

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner_window: Handle) -> Bool;
    fn CloseClipboard() -> Bool;
    fn IsClipboardFormatAvailable(format: u32) -> Bool;
    fn GetClipboardData(format: u32) -> Handle;
    fn EmptyClipboard() -> Bool;
    fn SetClipboardData(format: u32, memory: Handle) -> Handle;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> Handle;
    fn GlobalFree(memory: Handle) -> Handle;
    fn GlobalLock(memory: Handle) -> *mut c_void;
    fn GlobalUnlock(memory: Handle) -> Bool;
    fn GlobalSize(memory: Handle) -> usize;
}

/// A non-native error category used only inside the Windows adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ClipboardRawError {
    Unavailable,
    InvalidText,
    TooLarge,
}

pub(super) fn read_text(owner_window: Handle) -> Result<Option<String>, ClipboardRawError> {
    let _clipboard = ClipboardGuard::open(owner_window)?;
    let available = unsafe {
        // SAFETY: CF_UNICODETEXT is a constant Windows clipboard format and
        // querying it does not transfer or retain any native resource.
        IsClipboardFormatAvailable(CF_UNICODETEXT)
    };
    if available == 0 {
        return Ok(None);
    }

    let memory = unsafe {
        // SAFETY: the clipboard is open for this thread and the requested
        // format is fixed to CF_UNICODETEXT.
        GetClipboardData(CF_UNICODETEXT)
    };
    if memory == 0 {
        return Err(ClipboardRawError::Unavailable);
    }
    let size = unsafe {
        // SAFETY: GetClipboardData returned this HGLOBAL while the clipboard
        // remains open. GlobalSize only inspects that allocation.
        GlobalSize(memory)
    };
    if !(mem::size_of::<u16>()..=MAX_WINDOWS_TEXT_BYTES).contains(&size)
        || size % mem::size_of::<u16>() != 0
    {
        return Err(ClipboardRawError::TooLarge);
    }

    let locked = LockedMemory::lock(memory)?;
    decode_utf16(locked.as_u16_slice(size / mem::size_of::<u16>()))
}

pub(super) fn write_text(owner_window: Handle, text: &str) -> Result<(), ClipboardRawError> {
    let encoded = encode_utf16(text)?;
    let byte_len = encoded
        .len()
        .checked_mul(mem::size_of::<u16>())
        .ok_or(ClipboardRawError::TooLarge)?;
    let mut memory = GlobalMemory::allocate(byte_len)?;
    {
        let locked = LockedMemory::lock(memory.handle)?;
        unsafe {
            // SAFETY: GlobalAlloc reserved byte_len bytes and locked points to
            // that writable range. encoded remains alive for this synchronous
            // copy and contains exactly byte_len bytes.
            ptr::copy_nonoverlapping(
                encoded.as_ptr().cast::<u8>(),
                locked.pointer.cast::<u8>(),
                byte_len,
            );
        }
    }

    let _clipboard = ClipboardGuard::open(owner_window)?;
    let emptied = unsafe {
        // SAFETY: this thread owns an open clipboard. The full bounded value
        // has already been copied into a movable global-memory allocation.
        EmptyClipboard()
    };
    if emptied == 0 {
        return Err(ClipboardRawError::Unavailable);
    }
    let transferred = unsafe {
        // SAFETY: the clipboard is open and memory.handle owns the movable
        // global allocation required by SetClipboardData for CF_UNICODETEXT.
        SetClipboardData(CF_UNICODETEXT, memory.handle)
    };
    if transferred == 0 {
        return Err(ClipboardRawError::Unavailable);
    }
    memory.release_to_windows();
    Ok(())
}

struct ClipboardGuard;

impl ClipboardGuard {
    fn open(owner_window: Handle) -> Result<Self, ClipboardRawError> {
        let opened = unsafe {
            // SAFETY: owner_window is passed through only as the caller's
            // current native host window handle, or zero for no owner window.
            OpenClipboard(owner_window)
        };
        if opened == 0 {
            Err(ClipboardRawError::Unavailable)
        } else {
            Ok(Self)
        }
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: ClipboardGuard exists only after OpenClipboard succeeds
            // on this thread, and its drop closes that exact bounded scope.
            CloseClipboard();
        }
    }
}

struct GlobalMemory {
    handle: Handle,
}

impl GlobalMemory {
    fn allocate(bytes: usize) -> Result<Self, ClipboardRawError> {
        let handle = unsafe {
            // SAFETY: GMEM_MOVEABLE and the checked non-zero allocation size
            // meet GlobalAlloc's documented contract.
            GlobalAlloc(GMEM_MOVEABLE, bytes)
        };
        if handle == 0 {
            Err(ClipboardRawError::Unavailable)
        } else {
            Ok(Self { handle })
        }
    }

    fn release_to_windows(&mut self) {
        self.handle = 0;
    }
}

impl Drop for GlobalMemory {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                // SAFETY: GlobalMemory owns the allocation until a successful
                // SetClipboardData transfer clears the handle.
                GlobalFree(self.handle);
            }
        }
    }
}

struct LockedMemory {
    memory: Handle,
    pointer: *mut c_void,
}

impl LockedMemory {
    fn lock(memory: Handle) -> Result<Self, ClipboardRawError> {
        let pointer = unsafe {
            // SAFETY: memory is a live HGLOBAL from either GlobalAlloc or
            // GetClipboardData while its clipboard remains open.
            GlobalLock(memory)
        };
        if pointer.is_null() {
            Err(ClipboardRawError::Unavailable)
        } else {
            Ok(Self { memory, pointer })
        }
    }

    fn as_u16_slice(&self, length: usize) -> &[u16] {
        unsafe {
            // SAFETY: callers derive length from the validated GlobalSize for
            // this locked HGLOBAL, whose pointer stays valid until Drop.
            slice::from_raw_parts(self.pointer.cast::<u16>(), length)
        }
    }
}

impl Drop for LockedMemory {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: this guard represents one successful GlobalLock on the
            // live allocation held by the guard's owner.
            GlobalUnlock(self.memory);
        }
    }
}

fn encode_utf16(text: &str) -> Result<Vec<u16>, ClipboardRawError> {
    let mut encoded: Vec<u16> = text.encode_utf16().collect();
    encoded.push(0);
    let byte_len = encoded
        .len()
        .checked_mul(mem::size_of::<u16>())
        .ok_or(ClipboardRawError::TooLarge)?;
    if byte_len > MAX_WINDOWS_TEXT_BYTES {
        return Err(ClipboardRawError::TooLarge);
    }
    Ok(encoded)
}

fn decode_utf16(encoded: &[u16]) -> Result<Option<String>, ClipboardRawError> {
    let Some(terminator) = encoded.iter().position(|unit| *unit == 0) else {
        return Err(ClipboardRawError::InvalidText);
    };
    let value =
        String::from_utf16(&encoded[..terminator]).map_err(|_| ClipboardRawError::InvalidText)?;
    if value.len() > MAX_CLIPBOARD_TEXT_BYTES {
        return Err(ClipboardRawError::TooLarge);
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{ClipboardRawError, decode_utf16, encode_utf16};

    #[test]
    fn text_encoding_has_one_terminal_nul() {
        let encoded = encode_utf16("Anodrel \u{1f9ed}").expect("fixture text is valid");
        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|unit| **unit == 0).count(), 1);
    }

    #[test]
    fn decoding_uses_the_terminal_nul() {
        assert_eq!(
            decode_utf16(&['A' as u16, 0, 'B' as u16]),
            Ok(Some("A".to_owned()))
        );
    }

    #[test]
    fn malformed_or_unterminated_windows_text_is_rejected() {
        assert_eq!(
            decode_utf16(&[0xd800, 0]),
            Err(ClipboardRawError::InvalidText)
        );
        assert_eq!(
            decode_utf16(&['A' as u16]),
            Err(ClipboardRawError::InvalidText)
        );
    }
}
