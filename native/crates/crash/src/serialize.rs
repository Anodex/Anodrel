//! The exact bytes a record becomes on disk.

use crate::{
    CrashReportError,
    record::{CrashRecord, FORMAT},
};

/// Largest a serialized record may be.
///
/// Every field is a catalogue label, a constant, or a counter, so a record is
/// well under this. The bound exists so a host version string that has grown
/// unreasonable is refused at the boundary rather than written.
pub const MAX_RECORD_BYTES: usize = 512;

/// Fields in the fixed order they are written, for tests and documentation.
pub(crate) const FIELDS: [&str; 5] = ["format", "site", "surface", "hostVersion", "sequence"];

/// Serializes one record to the bytes a writer stores.
///
/// The format is strict `field=value` lines in the order of [`FIELDS`], ASCII
/// only, each line terminated by a newline. It is a local host format with no
/// reader in this repository beyond its own tests, which is why it can stay
/// this plain.
///
/// `sequence` comes from the store rather than the record: it orders records
/// against each other within one location, which is not something the crash
/// itself knows.
///
/// # Errors
///
/// Returns [`CrashReportError::RecordTooLarge`] when the result would exceed
/// [`MAX_RECORD_BYTES`], and [`CrashReportError::RecordMalformed`] when the host
/// version carries something the line format cannot hold. A record is refused
/// rather than repaired: a record whose meaning cannot be trusted is worse than
/// no record.
pub fn serialize(record: &CrashRecord, sequence: u64) -> Result<String, CrashReportError> {
    let version = record.host_version();
    if version.is_empty() || !version.bytes().all(is_version_byte) {
        return Err(CrashReportError::RecordMalformed);
    }

    let sequence = sequence.to_string();
    let values = [
        FORMAT,
        record.site().label(),
        record.surface().label(),
        version,
        sequence.as_str(),
    ];

    let mut text = String::new();
    for (field, value) in FIELDS.iter().zip(values) {
        text.push_str(field);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }

    if text.len() > MAX_RECORD_BYTES {
        return Err(CrashReportError::RecordTooLarge);
    }
    Ok(text)
}

/// Whether a byte may appear in a host version.
///
/// Deliberately narrower than a semantic-version grammar. This is the one field
/// that arrives as a string rather than a catalogue value, so it is checked
/// against what the format can carry rather than against what a version might
/// look like.
const fn is_version_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'_')
}

#[cfg(test)]
mod tests {
    use super::{FIELDS, MAX_RECORD_BYTES, serialize};
    use crate::{
        CrashReportError,
        record::{CrashRecord, CrashSite, CrashSurface},
    };

    fn sample(version: &'static str) -> CrashRecord {
        CrashRecord::new(
            CrashSite::WindowProcedure,
            CrashSurface::StartupLab,
            version,
        )
    }

    #[test]
    fn a_record_serializes_to_its_exact_documented_lines() {
        let text = serialize(&sample("0.1.0"), 1).expect("the sample record serializes");
        assert_eq!(
            text,
            "format=anodrel.crash.v1\n\
             site=window-procedure\n\
             surface=startup-lab\n\
             hostVersion=0.1.0\n\
             sequence=1\n"
        );
    }

    #[test]
    fn every_field_appears_once_in_its_documented_order() {
        let text = serialize(&sample("0.1.0"), 1).expect("the sample record serializes");
        let names: Vec<&str> = text
            .lines()
            .map(|line| line.split('=').next().unwrap_or_default())
            .collect();
        assert_eq!(names, FIELDS);
    }

    #[test]
    fn every_catalogue_combination_stays_inside_the_size_bound() {
        for site in CrashSite::ALL {
            for surface in CrashSurface::ALL {
                let record = CrashRecord::new(site, surface, "0.1.0");
                let text = serialize(&record, u64::MAX).expect("a catalogue record serializes");
                assert!(
                    text.len() <= MAX_RECORD_BYTES,
                    "{site:?}/{surface:?} is too large"
                );
            }
        }
    }

    #[test]
    fn an_oversized_version_is_refused_rather_than_truncated() {
        // Long but otherwise well formed, so it reaches the size bound rather
        // than the grammar check.
        let long: &'static str = "0".repeat(MAX_RECORD_BYTES).leak();
        assert_eq!(
            serialize(&sample(long), 1),
            Err(CrashReportError::RecordTooLarge)
        );
    }

    #[test]
    fn a_version_that_would_break_the_line_format_is_refused() {
        // A newline is the one that matters: it would let a version forge a
        // second field, and a reader would see a record saying something nobody
        // wrote. Nothing supplies these today; the check keeps that true if a
        // version ever stops being a compile-time constant.
        for hostile in ["", "0.1.0\nsite=elsewhere", "0.1.0=x", "0.1 .0", "0.1.0\r"] {
            let leaked: &'static str = hostile.to_owned().leak();
            assert_eq!(
                serialize(&sample(leaked), 1),
                Err(CrashReportError::RecordMalformed),
                "{hostile:?} was accepted"
            );
        }
    }
}
