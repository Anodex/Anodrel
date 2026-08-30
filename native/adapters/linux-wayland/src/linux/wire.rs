//! The small, strict subset of the Wayland wire format used by the lab.

const HEADER_BYTES: usize = 8;
const MAX_MESSAGE_BYTES: usize = 65_532;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WireError {
    Malformed,
    Oversized,
    ExhaustedIds,
}

pub(super) struct ObjectIds {
    next: u32,
}

impl ObjectIds {
    pub(super) const fn new() -> Self {
        Self { next: 2 }
    }

    pub(super) fn allocate(&mut self) -> Result<u32, WireError> {
        if self.next >= 0xfeff_ffff {
            return Err(WireError::ExhaustedIds);
        }
        let id = self.next;
        self.next += 1;
        Ok(id)
    }
}

pub(super) struct Request {
    object: u32,
    opcode: u16,
    bytes: Vec<u8>,
}

impl Request {
    pub(super) fn new(object: u32, opcode: u16) -> Self {
        Self {
            object,
            opcode,
            bytes: Vec::with_capacity(32),
        }
    }

    pub(super) fn uint(&mut self, value: u32) {
        self.bytes.extend(value.to_ne_bytes());
    }

    pub(super) fn int(&mut self, value: i32) {
        self.bytes.extend(value.to_ne_bytes());
    }

    pub(super) fn string(&mut self, value: &str) -> Result<(), WireError> {
        if value.as_bytes().contains(&0) {
            return Err(WireError::Malformed);
        }
        let length = value.len().checked_add(1).ok_or(WireError::Oversized)?;
        self.uint(u32::try_from(length).map_err(|_| WireError::Oversized)?);
        self.bytes.extend(value.bytes());
        self.bytes.push(0);
        self.pad();
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Vec<u8>, WireError> {
        self.pad();
        let size = HEADER_BYTES
            .checked_add(self.bytes.len())
            .filter(|size| *size <= MAX_MESSAGE_BYTES)
            .ok_or(WireError::Oversized)?;
        let mut message = Vec::with_capacity(size);
        message.extend(self.object.to_ne_bytes());
        let word =
            (u32::try_from(size).map_err(|_| WireError::Oversized)? << 16) | u32::from(self.opcode);
        message.extend(word.to_ne_bytes());
        message.extend(self.bytes);
        Ok(message)
    }

    fn pad(&mut self) {
        self.bytes.resize((self.bytes.len() + 3) & !3, 0);
    }
}

pub(super) struct Message {
    pub(super) object: u32,
    pub(super) opcode: u16,
    body: Vec<u8>,
}

impl Message {
    pub(super) fn from_frame(frame: &[u8]) -> Result<Self, WireError> {
        if frame.len() < HEADER_BYTES
            || frame.len() > MAX_MESSAGE_BYTES
            || !frame.len().is_multiple_of(4)
        {
            return Err(WireError::Malformed);
        }
        let object = u32::from_ne_bytes(frame[0..4].try_into().map_err(|_| WireError::Malformed)?);
        let word = u32::from_ne_bytes(frame[4..8].try_into().map_err(|_| WireError::Malformed)?);
        let size = (word >> 16) as usize;
        if size != frame.len() || object == 0 {
            return Err(WireError::Malformed);
        }
        Ok(Self {
            object,
            opcode: word as u16,
            body: frame[HEADER_BYTES..].to_vec(),
        })
    }

    pub(super) fn reader(&self) -> Reader<'_> {
        Reader {
            body: &self.body,
            offset: 0,
        }
    }
}

pub(super) struct Reader<'a> {
    body: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn uint(&mut self) -> Result<u32, WireError> {
        let bytes = self.take(4)?;
        Ok(u32::from_ne_bytes(
            bytes.try_into().map_err(|_| WireError::Malformed)?,
        ))
    }

    pub(super) fn int(&mut self) -> Result<i32, WireError> {
        let bytes = self.take(4)?;
        Ok(i32::from_ne_bytes(
            bytes.try_into().map_err(|_| WireError::Malformed)?,
        ))
    }

    pub(super) fn string(&mut self) -> Result<&'a str, WireError> {
        let length = usize::try_from(self.uint()?).map_err(|_| WireError::Malformed)?;
        if length == 0 {
            return Err(WireError::Malformed);
        }
        let bytes = self.take(length)?;
        if bytes.last() != Some(&0) {
            return Err(WireError::Malformed);
        }
        let text = std::str::from_utf8(&bytes[..length - 1]).map_err(|_| WireError::Malformed)?;
        self.skip_padding(length)?;
        Ok(text)
    }

    pub(super) fn array(&mut self) -> Result<&'a [u8], WireError> {
        let length = usize::try_from(self.uint()?).map_err(|_| WireError::Malformed)?;
        let bytes = self.take(length)?;
        self.skip_padding(length)?;
        Ok(bytes)
    }

    pub(super) fn finish(self) -> Result<(), WireError> {
        self.offset
            .eq(&self.body.len())
            .then_some(())
            .ok_or(WireError::Malformed)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], WireError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(WireError::Malformed)?;
        let bytes = self
            .body
            .get(self.offset..end)
            .ok_or(WireError::Malformed)?;
        self.offset = end;
        Ok(bytes)
    }

    fn skip_padding(&mut self, length: usize) -> Result<(), WireError> {
        let padding = ((length + 3) & !3) - length;
        if self.take(padding)?.iter().any(|byte| *byte != 0) {
            return Err(WireError::Malformed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, ObjectIds, Request, WireError};

    #[test]
    fn encodes_a_padded_string_request_with_the_wayland_header() {
        let mut request = Request::new(7, 2);
        request.string("Lab").expect("fixture string is valid");
        let encoded = request.finish().expect("request fits");
        assert_eq!(&encoded[0..4], &7_u32.to_ne_bytes());
        assert_eq!(&encoded[4..8], &[2, 0, 16, 0]);
        assert_eq!(&encoded[8..12], &4_u32.to_ne_bytes());
        assert_eq!(&encoded[12..16], b"Lab\0");
    }

    #[test]
    fn rejects_malformed_string_padding_and_trailing_fields() {
        let frame = [2, 0, 0, 0, 0, 0, 16, 0, 3, 0, 0, 0, b'a', b'\0', 7, 0];
        let message = Message::from_frame(&frame).expect("header is structurally valid");
        assert!(message.reader().string().is_err());
        assert!(Message::from_frame(&[0; 8]).is_err());
    }

    #[test]
    fn client_ids_are_dense_and_never_use_display_id() {
        let mut ids = ObjectIds::new();
        assert_eq!(ids.allocate(), Ok(2));
        assert_eq!(ids.allocate(), Ok(3));
        let mut exhausted = ObjectIds { next: 0xfeff_ffff };
        assert_eq!(exhausted.allocate(), Err(WireError::ExhaustedIds));
    }
}
