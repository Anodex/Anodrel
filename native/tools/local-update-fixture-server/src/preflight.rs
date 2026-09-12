//! Fixed signed-publication checks that run before the listener binds.

use std::{fs::File, io::Read, path::Path};

use anodrel_local_update_fixture::{
    APPLICATION_ID, FixturePublication, INSTALLER_REQUEST_TARGET, LOCALHOST, PORT,
};
use anodrel_windows_installer::{
    PackageVersion, VerifiedInstallerImage, verify_locked_installer_image,
};
use anodrel_windows_signature::verify_embedded_signature;
use anodrel_windows_update_catalogue_signature::{
    MAX_SIGNED_UPDATE_CATALOGUE_BYTES, verify_update_catalogue,
};

/// Validates the fixed catalogue and retains the locked candidate installer.
pub(crate) fn verify(publication: &FixturePublication) -> Result<VerifiedInstallerImage, ()> {
    let (catalogue_path, catalogue_size) = publication.catalogue();
    let (installer_path, installer_size) = publication.installer();
    let image = verify_locked_installer_image(installer_path).map_err(|_| ())?;
    let signer = verify_embedded_signature(installer_path).map_err(|_| ())?;
    let envelope = read_catalogue(catalogue_path, catalogue_size)?;
    let catalogue = verify_update_catalogue(&envelope, signer.as_bytes()).map_err(|_| ())?;
    let installer = catalogue.installer();

    (catalogue.application_id() == APPLICATION_ID
        && catalogue.package_version() == PackageVersion::new(0, 1, 1)
        && catalogue.matches_installed(APPLICATION_ID, signer.as_bytes())
        && catalogue.matches_release(image.manifest())
        && installer.byte_length() == installer_size
        && has_fixed_installer_location(
            installer.origin().hostname(),
            installer.origin().port(),
            installer.request_path(),
        ))
    .then_some(image)
    .ok_or(())
}

fn read_catalogue(path: &Path, expected_size: u64) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::with_capacity(expected_size as usize);
    File::open(path)
        .map_err(|_| ())?
        .take(MAX_SIGNED_UPDATE_CATALOGUE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    (bytes.len() <= MAX_SIGNED_UPDATE_CATALOGUE_BYTES && bytes.len() as u64 == expected_size)
        .then_some(bytes)
        .ok_or(())
}

fn has_fixed_installer_location(host: &str, port: u16, path: &str) -> bool {
    host == LOCALHOST && port == PORT && path == INSTALLER_REQUEST_TARGET
}

#[cfg(test)]
mod tests {
    use super::has_fixed_installer_location;
    use anodrel_local_update_fixture::{INSTALLER_REQUEST_TARGET, LOCALHOST, PORT};

    #[test]
    fn only_the_fixed_fixture_installer_location_is_accepted() {
        assert!(has_fixed_installer_location(
            LOCALHOST,
            PORT,
            INSTALLER_REQUEST_TARGET
        ));
        for location in [
            ("127.0.0.1", PORT, INSTALLER_REQUEST_TARGET),
            (LOCALHOST, 443, INSTALLER_REQUEST_TARGET),
            (
                LOCALHOST,
                PORT,
                "/anodrel/local-update/releases/0.1.0/installer.exe",
            ),
        ] {
            assert!(!has_fixed_installer_location(
                location.0, location.1, location.2
            ));
        }
    }
}
