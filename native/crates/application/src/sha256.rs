//! Small, allocation-free SHA-256 implementation built into Anodrel.
//!
//! This module is public so a host-side provisioning tool can compute the exact
//! digest the record parser will later compare against, using the same code
//! rather than a second implementation. It is a pure hash: it selects no policy,
//! reads no path of its own, and grants no authority.

use std::io::{self, Read};

const INITIAL_STATE: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

const ROUND_CONSTANTS: [u32; 64] = [
    0x428A_2F98,
    0x7137_4491,
    0xB5C0_FBCF,
    0xE9B5_DBA5,
    0x3956_C25B,
    0x59F1_11F1,
    0x923F_82A4,
    0xAB1C_5ED5,
    0xD807_AA98,
    0x1283_5B01,
    0x2431_85BE,
    0x550C_7DC3,
    0x72BE_5D74,
    0x80DE_B1FE,
    0x9BDC_06A7,
    0xC19B_F174,
    0xE49B_69C1,
    0xEFBE_4786,
    0x0FC1_9DC6,
    0x240C_A1CC,
    0x2DE9_2C6F,
    0x4A74_84AA,
    0x5CB0_A9DC,
    0x76F9_88DA,
    0x983E_5152,
    0xA831_C66D,
    0xB003_27C8,
    0xBF59_7FC7,
    0xC6E0_0BF3,
    0xD5A7_9147,
    0x06CA_6351,
    0x1429_2967,
    0x27B7_0A85,
    0x2E1B_2138,
    0x4D2C_6DFC,
    0x5338_0D13,
    0x650A_7354,
    0x766A_0ABB,
    0x81C2_C92E,
    0x9272_2C85,
    0xA2BF_E8A1,
    0xA81A_664B,
    0xC24B_8B70,
    0xC76C_51A3,
    0xD192_E819,
    0xD699_0624,
    0xF40E_3585,
    0x106A_A070,
    0x19A4_C116,
    0x1E37_6C08,
    0x2748_774C,
    0x34B0_BCB5,
    0x391C_0CB3,
    0x4ED8_AA4A,
    0x5B9C_CA4F,
    0x682E_6FF3,
    0x748F_82EE,
    0x78A5_636F,
    0x84C8_7814,
    0x8CC7_0208,
    0x90BE_FFFA,
    0xA450_6CEB,
    0xBEF9_A3F7,
    0xC671_78F2,
];

pub fn digest(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finish()
}

/// Renders a digest as lower-case hexadecimal.
pub fn to_lower_hex(digest: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0F)] as char);
    }
    out
}

pub fn parse_lower_hex(input: &str) -> Option<[u8; 32]> {
    if input.len() != 64 {
        return None;
    }

    let mut output = [0_u8; 32];
    for (index, pair) in input.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    Some(output)
}

/// Hashes a reader while stopping as soon as the configured byte limit is
/// exceeded. The `None` result means the caller's limit was exceeded.
pub fn digest_reader_limited<R: Read>(
    reader: &mut R,
    maximum: usize,
) -> io::Result<Option<([u8; 32], usize)>> {
    let mut hasher = Sha256::new();
    let mut total = 0_usize;
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(Some((hasher.finish(), total)));
        }
        total = total
            .checked_add(read)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "input length overflowed"))?;
        if total > maximum {
            return Ok(None);
        }
        hasher.update(&buffer[..read]);
    }
}

#[cfg(test)]
pub(crate) fn lower_hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(DIGITS[usize::from(byte >> 4)] as char);
        output.push(DIGITS[usize::from(byte & 0x0F)] as char);
    }
    output
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

struct Sha256 {
    state: [u32; 8],
    total_bytes: u64,
    buffer: [u8; 64],
    buffered: usize,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            total_bytes: 0,
            buffer: [0; 64],
            buffered: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.total_bytes = self.total_bytes.wrapping_add(input.len() as u64);

        if self.buffered != 0 {
            let needed = 64 - self.buffered;
            let copied = needed.min(input.len());
            self.buffer[self.buffered..self.buffered + copied].copy_from_slice(&input[..copied]);
            self.buffered += copied;
            input = &input[copied..];
            if self.buffered == 64 {
                transform(&mut self.state, &self.buffer);
                self.buffered = 0;
            }
        }

        while input.len() >= 64 {
            let block = input[..64].try_into().expect("chunk has exactly 64 bytes");
            transform(&mut self.state, &block);
            input = &input[64..];
        }

        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffered = input.len();
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_length = self.total_bytes.wrapping_mul(8);
        let padding = if self.buffered < 56 {
            56 - self.buffered
        } else {
            120 - self.buffered
        };
        self.update(&[0x80]);
        self.update(&[0; 64][..padding - 1]);
        self.update(&bit_length.to_be_bytes());

        debug_assert_eq!(self.buffered, 0);
        let mut output = [0_u8; 32];
        for (index, value) in self.state.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        output
    }
}

fn transform(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut schedule = [0_u32; 64];
    for (index, word) in schedule[..16].iter_mut().enumerate() {
        *word = u32::from_be_bytes(
            block[index * 4..index * 4 + 4]
                .try_into()
                .expect("block slice has exactly four bytes"),
        );
    }
    for index in 16..64 {
        let first = schedule[index - 15];
        let second = schedule[index - 2];
        let small_sigma0 = first.rotate_right(7) ^ first.rotate_right(18) ^ (first >> 3);
        let small_sigma1 = second.rotate_right(17) ^ second.rotate_right(19) ^ (second >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(small_sigma0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(small_sigma1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for index in 0..64 {
        let big_sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ ((!e) & g);
        let temporary1 = h
            .wrapping_add(big_sigma1)
            .wrapping_add(choice)
            .wrapping_add(ROUND_CONSTANTS[index])
            .wrapping_add(schedule[index]);
        let big_sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temporary2 = big_sigma0.wrapping_add(majority);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temporary1);
        d = c;
        c = b;
        b = a;
        a = temporary1.wrapping_add(temporary2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

#[cfg(test)]
mod tests {
    use super::{digest, lower_hex, parse_lower_hex};

    #[test]
    fn matches_standard_sha256_vectors() {
        assert_eq!(
            lower_hex(&digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            lower_hex(&digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_multiple_blocks_and_only_accepts_lower_hex() {
        let input = vec![b'a'; 1_000_000];
        assert_eq!(
            lower_hex(&digest(&input)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert!(
            parse_lower_hex("BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD")
                .is_none()
        );
        assert_eq!(
            parse_lower_hex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            Some(digest(b"abc"))
        );
    }
}
