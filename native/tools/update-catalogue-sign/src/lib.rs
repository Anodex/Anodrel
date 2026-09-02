#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! First-party fresh-output authoring for signed Windows update catalogues.
//!
//! This tool verifies strict input and uses an exact current-user certificate
//! to create one attached CMS output. It does not retrieve an update, install,
//! launch, elevate, change trust, or modify an input file.

mod error;
mod output;

use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use anodrel_application::sha256;
use anodrel_update_catalogue::MAX_UPDATE_CATALOGUE_BYTES;
use anodrel_windows_update_catalogue_signature::sign_update_catalogue;

pub use error::UpdateCatalogueSignToolError;

/// Creates one fresh attached-CMS output from one exact update catalogue.
///
/// Every path must be absolute. The input is one normal regular UTF-8 file
/// within the catalogue limit, and the output is a previously absent file with
/// a normal existing parent. The certificate fingerprint is lowercase SHA-256
/// text for one certificate in the current user's Windows `MY` store.
pub fn sign_catalogue_file(
    input: &Path,
    certificate_fingerprint: &str,
    output: &Path,
) -> Result<(), UpdateCatalogueSignToolError> {
    let fingerprint = sha256::parse_lower_hex(certificate_fingerprint)
        .ok_or(UpdateCatalogueSignToolError::CertificateFingerprintInvalid)?;
    output::validate_output(output)?;
    let input = read_input(input)?;
    let text =
        std::str::from_utf8(&input).map_err(|_| UpdateCatalogueSignToolError::InputInvalid)?;
    let signed = sign_update_catalogue(text, fingerprint)
        .map_err(UpdateCatalogueSignToolError::SignatureInvalid)?;
    output::write_new(output, &signed)
}

fn read_input(path: &Path) -> Result<Vec<u8>, UpdateCatalogueSignToolError> {
    if !path.is_absolute() {
        return Err(UpdateCatalogueSignToolError::InputInvalid);
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| UpdateCatalogueSignToolError::InputReadFailed)?;
    if !metadata.is_file() || is_link_like(&metadata) {
        return Err(UpdateCatalogueSignToolError::InputInvalid);
    }
    if metadata.len() > MAX_UPDATE_CATALOGUE_BYTES as u64 {
        return Err(UpdateCatalogueSignToolError::InputReadFailed);
    }
    let file = File::open(path).map_err(|_| UpdateCatalogueSignToolError::InputReadFailed)?;
    let capacity = usize::try_from(metadata.len())
        .map_err(|_| UpdateCatalogueSignToolError::InputReadFailed)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_UPDATE_CATALOGUE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| UpdateCatalogueSignToolError::InputReadFailed)?;
    if bytes.len() > MAX_UPDATE_CATALOGUE_BYTES {
        return Err(UpdateCatalogueSignToolError::InputReadFailed);
    }
    Ok(bytes)
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
mod tests;
