#![forbid(unsafe_code)]

//! Bounded byte-stream framing for the Anodrel JSON protocol.
//!
//! This crate performs only framing and UTF-8 validation. Protocol validation
//! and capability policy intentionally remain in higher layers.

use std::fmt;

pub const MAGIC: [u8; 4] = *b"ANDR";
pub const WIRE_MAJOR: u16 = 1;
pub const WIRE_MINOR: u16 = 0;
pub const HEADER_BYTES: usize = 12;
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_FRAMES_PER_RECEIVE: usize = 4;
pub const MAX_BUFFERED_BYTES: usize = MAX_FRAMES_PER_RECEIVE * (HEADER_BYTES + MAX_PAYLOAD_BYTES);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WireError {
    BufferLimitExceeded,
    InvalidMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    PayloadTooLarge { declared_bytes: usize },
    InvalidUtf8,
    ReceiveBurstLimitExceeded,
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferLimitExceeded => write!(formatter, "wire receive buffer limit exceeded"),
            Self::InvalidMagic => write!(formatter, "wire frame magic is invalid"),
            Self::UnsupportedVersion { major, minor } => {
                write!(formatter, "wire version {major}.{minor} is not supported")
            }
            Self::PayloadTooLarge { declared_bytes } => {
                write!(
                    formatter,
                    "wire payload declares {declared_bytes} bytes, exceeding the limit"
                )
            }
            Self::InvalidUtf8 => write!(formatter, "wire payload is not valid UTF-8"),
            Self::ReceiveBurstLimitExceeded => {
                write!(formatter, "wire receive burst contains too many frames")
            }
        }
    }
}

impl std::error::Error for WireError {}

#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffered: Vec<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds stream bytes and returns every complete UTF-8 JSON payload now
    /// available. Any frame error invalidates the current stream buffer; an OS
    /// adapter must close the owning session instead of resynchronizing.
    pub fn push(&mut self, input: &[u8]) -> Result<Vec<String>, WireError> {
        if input.len() > MAX_BUFFERED_BYTES.saturating_sub(self.buffered.len()) {
            return self.abort(WireError::BufferLimitExceeded);
        }
        self.buffered.extend_from_slice(input);

        let mut messages = Vec::new();
        let mut consumed = 0_usize;
        loop {
            let available = self.buffered.len() - consumed;
            if available < HEADER_BYTES {
                break;
            }

            let header = &self.buffered[consumed..consumed + HEADER_BYTES];
            if header[..4] != MAGIC {
                return self.abort(WireError::InvalidMagic);
            }
            let major = u16::from_le_bytes([header[4], header[5]]);
            let minor = u16::from_le_bytes([header[6], header[7]]);
            if (major, minor) != (WIRE_MAJOR, WIRE_MINOR) {
                return self.abort(WireError::UnsupportedVersion { major, minor });
            }
            let payload_bytes =
                u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
            if payload_bytes > MAX_PAYLOAD_BYTES {
                return self.abort(WireError::PayloadTooLarge {
                    declared_bytes: payload_bytes,
                });
            }
            let frame_bytes = HEADER_BYTES + payload_bytes;
            if available < frame_bytes {
                break;
            }
            if messages.len() == MAX_FRAMES_PER_RECEIVE {
                return self.abort(WireError::ReceiveBurstLimitExceeded);
            }

            let payload_start = consumed + HEADER_BYTES;
            let payload_end = consumed + frame_bytes;
            let payload = match std::str::from_utf8(&self.buffered[payload_start..payload_end]) {
                Ok(payload) => payload.to_owned(),
                Err(_) => return self.abort(WireError::InvalidUtf8),
            };
            messages.push(payload);
            consumed = payload_end;
        }

        if consumed != 0 {
            self.buffered.drain(..consumed);
        }
        Ok(messages)
    }

    fn abort<T>(&mut self, error: WireError) -> Result<T, WireError> {
        self.buffered.clear();
        Err(error)
    }
}

pub fn encode_json(payload: &str) -> Result<Vec<u8>, WireError> {
    let payload_bytes = payload.len();
    if payload_bytes > MAX_PAYLOAD_BYTES {
        return Err(WireError::PayloadTooLarge {
            declared_bytes: payload_bytes,
        });
    }
    let mut frame = Vec::with_capacity(HEADER_BYTES + payload_bytes);
    frame.extend_from_slice(&MAGIC);
    frame.extend_from_slice(&WIRE_MAJOR.to_le_bytes());
    frame.extend_from_slice(&WIRE_MINOR.to_le_bytes());
    frame.extend_from_slice(&(payload_bytes as u32).to_le_bytes());
    frame.extend_from_slice(payload.as_bytes());
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_a_fragmented_frame() {
        let frame = encode_json(r#"{"requestId":"one"}"#).expect("frame encodes");
        let mut decoder = FrameDecoder::new();
        assert!(
            decoder
                .push(&frame[..7])
                .expect("fragment is valid")
                .is_empty()
        );
        assert_eq!(
            decoder.push(&frame[7..]).expect("frame completes"),
            vec![r#"{"requestId":"one"}"#]
        );
    }

    #[test]
    fn separates_coalesced_frames_without_losing_order() {
        let mut input = encode_json(r#"{"requestId":"one"}"#).expect("first frame encodes");
        input.extend(encode_json(r#"{"requestId":"two"}"#).expect("second frame encodes"));
        assert_eq!(
            FrameDecoder::new().push(&input).expect("frames decode"),
            vec![r#"{"requestId":"one"}"#, r#"{"requestId":"two"}"#]
        );
    }

    #[test]
    fn rejects_declared_oversize_before_payload_arrives() {
        let mut frame = Vec::from(MAGIC);
        frame.extend_from_slice(&WIRE_MAJOR.to_le_bytes());
        frame.extend_from_slice(&WIRE_MINOR.to_le_bytes());
        frame.extend_from_slice(&((MAX_PAYLOAD_BYTES + 1) as u32).to_le_bytes());
        assert_eq!(
            FrameDecoder::new().push(&frame),
            Err(WireError::PayloadTooLarge {
                declared_bytes: MAX_PAYLOAD_BYTES + 1
            })
        );
    }

    #[test]
    fn rejects_invalid_utf8_and_excessive_bursts() {
        let mut invalid = encode_json("a").expect("frame encodes");
        invalid[HEADER_BYTES] = 0xFF;
        assert_eq!(
            FrameDecoder::new().push(&invalid),
            Err(WireError::InvalidUtf8)
        );

        let mut burst = Vec::new();
        for _ in 0..=MAX_FRAMES_PER_RECEIVE {
            burst.extend(encode_json("{}").expect("frame encodes"));
        }
        assert_eq!(
            FrameDecoder::new().push(&burst),
            Err(WireError::ReceiveBurstLimitExceeded)
        );
    }

    #[test]
    fn rejects_input_that_would_exceed_the_buffer_limit() {
        assert_eq!(
            FrameDecoder::new().push(&vec![0; MAX_BUFFERED_BYTES + 1]),
            Err(WireError::BufferLimitExceeded)
        );
    }
}
