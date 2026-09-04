//! Bounded canonical binary data for one selected-output replacement.

use std::fmt;

/// Maximum decoded bytes a single binary output request may carry.
pub const MAX_FILE_BINARY_WRITE_BYTES: usize = 32 * 1024;

/// One bounded sequence of decoded bytes for a selected-output write.
///
/// This type can be created only from a bounded byte vector or a canonical
/// unpadded base64url value. It keeps codec validation out of native adapters.
#[derive(Clone, Eq, PartialEq)]
pub struct FileBinaryData(Vec<u8>);

impl FileBinaryData {
    /// Validates and retains one sequence of decoded output bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, FileBinaryDataError> {
        if bytes.len() > MAX_FILE_BINARY_WRITE_BYTES {
            return Err(FileBinaryDataError::TooLarge);
        }
        Ok(Self(bytes))
    }

    /// Decodes one unpadded canonical base64url value.
    ///
    /// The decoder rejects padding, whitespace, malformed lengths, and a
    /// non-zero unused tail so one byte sequence has one accepted spelling.
    pub fn decode_base64url(value: &str) -> Result<Self, FileBinaryDataError> {
        let bytes = value.as_bytes();
        if bytes.iter().any(|byte| base64_value(*byte).is_none()) {
            return Err(FileBinaryDataError::Invalid);
        }
        let remainder = bytes.len() % 4;
        if remainder == 1 {
            return Err(FileBinaryDataError::Invalid);
        }
        if let Some(last) = bytes.last().and_then(|byte| base64_value(*byte)) {
            let tail_is_canonical = match remainder {
                2 => last & 0b0000_1111 == 0,
                3 => last & 0b0000_0011 == 0,
                _ => true,
            };
            if !tail_is_canonical {
                return Err(FileBinaryDataError::Invalid);
            }
        }

        let decoded_length = (bytes.len() / 4)
            .checked_mul(3)
            .and_then(|length| match remainder {
                0 => Some(length),
                2 => length.checked_add(1),
                3 => length.checked_add(2),
                _ => None,
            })
            .ok_or(FileBinaryDataError::TooLarge)?;
        if decoded_length > MAX_FILE_BINARY_WRITE_BYTES {
            return Err(FileBinaryDataError::TooLarge);
        }

        let mut decoded = Vec::with_capacity(decoded_length);
        let complete = bytes.len() - remainder;
        let mut index = 0;
        while index < complete {
            let first = required_value(bytes[index]);
            let second = required_value(bytes[index + 1]);
            let third = required_value(bytes[index + 2]);
            let fourth = required_value(bytes[index + 3]);
            decoded.push((first << 2) | (second >> 4));
            decoded.push((second << 4) | (third >> 2));
            decoded.push((third << 6) | fourth);
            index += 4;
        }
        if remainder >= 2 {
            let first = required_value(bytes[index]);
            let second = required_value(bytes[index + 1]);
            decoded.push((first << 2) | (second >> 4));
            if remainder == 3 {
                let third = required_value(bytes[index + 2]);
                decoded.push((second << 4) | (third >> 2));
            }
        }

        debug_assert_eq!(decoded.len(), decoded_length);
        Ok(Self(decoded))
    }

    /// Returns the exact bounded bytes for the native output adapter.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Encodes these bounded bytes as exact unpadded canonical base64url.
    ///
    /// The protocol carries binary data as JSON text. Keeping the encoder next
    /// to the validating decoder lets typed clients use one representation
    /// without a third-party codec or a second wire format.
    #[must_use]
    pub fn to_base64url(&self) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

        let mut encoded = String::with_capacity((self.0.len() * 4).div_ceil(3));
        let mut index = 0;
        while index + 3 <= self.0.len() {
            let first = self.0[index];
            let second = self.0[index + 1];
            let third = self.0[index + 2];
            encoded.push(char::from(ALPHABET[usize::from(first >> 2)]));
            encoded.push(char::from(
                ALPHABET[usize::from((first & 0b11) << 4 | second >> 4)],
            ));
            encoded.push(char::from(
                ALPHABET[usize::from((second & 0b1111) << 2 | third >> 6)],
            ));
            encoded.push(char::from(ALPHABET[usize::from(third & 0b11_1111)]));
            index += 3;
        }
        match self.0.len() - index {
            0 => {}
            1 => {
                let first = self.0[index];
                encoded.push(char::from(ALPHABET[usize::from(first >> 2)]));
                encoded.push(char::from(ALPHABET[usize::from((first & 0b11) << 4)]));
            }
            2 => {
                let first = self.0[index];
                let second = self.0[index + 1];
                encoded.push(char::from(ALPHABET[usize::from(first >> 2)]));
                encoded.push(char::from(
                    ALPHABET[usize::from((first & 0b11) << 4 | second >> 4)],
                ));
                encoded.push(char::from(ALPHABET[usize::from((second & 0b1111) << 2)]));
            }
            _ => unreachable!("the loop retains at most two source bytes"),
        }
        encoded
    }
}

impl fmt::Debug for FileBinaryData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FileBinaryData(..)")
    }
}

/// A safe binary-data validation failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileBinaryDataError {
    /// The encoded representation was not exact canonical base64url.
    Invalid,
    /// The decoded value would exceed the fixed request bound.
    TooLarge,
}

impl fmt::Display for FileBinaryDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("binary output data is invalid")
    }
}

impl std::error::Error for FileBinaryDataError {}

const fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn required_value(byte: u8) -> u8 {
    // The complete input was validated before decoding.
    base64_value(byte).expect("validated base64url value")
}

#[cfg(test)]
mod tests {
    use super::{FileBinaryData, FileBinaryDataError, MAX_FILE_BINARY_WRITE_BYTES};

    #[test]
    fn decodes_empty_and_canonical_unpadded_base64url() {
        assert_eq!(
            FileBinaryData::decode_base64url("")
                .expect("empty value is valid")
                .as_bytes(),
            []
        );
        assert_eq!(
            FileBinaryData::decode_base64url("AAEC_w")
                .expect("value is valid")
                .as_bytes(),
            [0, 1, 2, 255]
        );
    }

    #[test]
    fn encodes_one_canonical_base64url_spelling_for_each_value() {
        for bytes in [vec![], vec![0], vec![0, 1], vec![0, 1, 2, 255]] {
            let data = FileBinaryData::from_bytes(bytes.clone()).expect("fixed bytes are bounded");
            let encoded = data.to_base64url();
            assert_eq!(
                FileBinaryData::decode_base64url(&encoded)
                    .expect("the produced spelling is canonical")
                    .as_bytes(),
                bytes
            );
        }
    }

    #[test]
    fn rejects_padding_noncanonical_tails_and_malformed_lengths() {
        for value in ["A", "AA=", "AA==", "AA ", "AB", "AAB"] {
            assert_eq!(
                FileBinaryData::decode_base64url(value),
                Err(FileBinaryDataError::Invalid),
                "{value:?} must not be accepted"
            );
        }
    }

    #[test]
    fn keeps_the_decoded_bound_even_for_a_canonical_value() {
        let encoded = "AAAA".repeat((MAX_FILE_BINARY_WRITE_BYTES / 3) + 1);
        assert_eq!(
            FileBinaryData::decode_base64url(&encoded),
            Err(FileBinaryDataError::TooLarge)
        );
    }

    #[test]
    fn rejects_an_unbounded_native_byte_vector() {
        assert_eq!(
            FileBinaryData::from_bytes(vec![0; MAX_FILE_BINARY_WRITE_BYTES + 1]),
            Err(FileBinaryDataError::TooLarge)
        );
    }
}
