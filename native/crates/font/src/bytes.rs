//! Checked big-endian views over untrusted SFNT bytes.

/// A slice view whose offset calculations remain bounded by its source bytes.
#[derive(Clone, Copy)]
pub(crate) struct Bytes<'font> {
    source: &'font [u8],
}

impl<'font> Bytes<'font> {
    /// Wraps an existing byte slice without copying it.
    pub(crate) const fn new(source: &'font [u8]) -> Self {
        Self { source }
    }

    /// Reads one big-endian unsigned 16-bit value.
    pub(crate) fn u16(self, offset: usize) -> Option<u16> {
        let bytes = self.range(offset, 2)?;
        Some(u16::from_be_bytes([bytes.source[0], bytes.source[1]]))
    }

    /// Reads one unsigned byte.
    pub(crate) fn u8(self, offset: usize) -> Option<u8> {
        self.source.get(offset).copied()
    }

    /// Reads one big-endian signed 16-bit value.
    pub(crate) fn i16(self, offset: usize) -> Option<i16> {
        self.u16(offset).map(|value| value as i16)
    }

    /// Reads one big-endian unsigned 32-bit value.
    pub(crate) fn u32(self, offset: usize) -> Option<u32> {
        let bytes = self.range(offset, 4)?;
        Some(u32::from_be_bytes([
            bytes.source[0],
            bytes.source[1],
            bytes.source[2],
            bytes.source[3],
        ]))
    }

    /// Borrows an exactly bounded nested view.
    pub(crate) fn range(self, offset: usize, length: usize) -> Option<Self> {
        let end = offset.checked_add(length)?;
        Some(Self::new(self.source.get(offset..end)?))
    }

    /// Returns the length without exposing the borrowed byte contents.
    pub(crate) const fn len(self) -> usize {
        self.source.len()
    }

    /// Returns whether a final unused range contains only zero padding.
    pub(crate) fn zero_padding_from(self, offset: usize) -> bool {
        self.source
            .get(offset..)
            .is_some_and(|bytes| bytes.iter().all(|byte| *byte == 0))
    }
}
