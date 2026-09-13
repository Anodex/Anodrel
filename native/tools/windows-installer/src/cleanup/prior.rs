//! Verified removal of the one retained rollback package after uninstall.

use std::path::Path;

use anodrel_windows_policy::{PolicyStoreError, load_previous_installed_application};
use anodrel_windows_signature::verify_embedded_signature;

use crate::{PackageVersion, recovery::raw::remove_normal_tree};

use super::CleanupError;

/// Removes only the one signed, retained prior package after policy removal.
///
/// The private `previous` record remains the sole authority for this second
/// version directory. A directory name, cache entry, or caller never selects
/// a package. Missing prior policy is normal for an application never updated.
pub(super) fn remove_verified_prior(
    root: &Path,
    application_id: &str,
    publisher: [u8; 32],
    current_version: PackageVersion,
) -> Result<(), CleanupError> {
    let prior = match load_previous_installed_application(application_id) {
        Ok(prior) => prior,
        Err(PolicyStoreError::RecordNotFound) => return Ok(()),
        Err(_) => return Err(CleanupError::Verification),
    };
    let prior_version =
        direct_version(root, prior.package_root()).ok_or(CleanupError::Verification)?;
    if prior_version >= current_version {
        return Err(CleanupError::Verification);
    }
    let signer = verify_embedded_signature(prior.executable_path())
        .map_err(|_| CleanupError::Verification)?;
    if !prior.matches_publisher(signer.as_bytes()) || signer.as_bytes() != publisher {
        return Err(CleanupError::Verification);
    }
    remove_normal_tree(prior.package_root()).map_err(|_| CleanupError::Package)
}

fn direct_version(root: &Path, package_root: &Path) -> Option<PackageVersion> {
    (package_root.parent()? == root)
        .then(|| package_root.file_name()?.to_str())?
        .and_then(PackageVersion::from_canonical_directory_name)
}

#[cfg(test)]
mod tests {
    use super::direct_version;

    #[test]
    fn only_a_direct_canonical_child_can_name_the_retained_package() {
        let root =
            std::path::Path::new(r"C:\\Program Files\\Anodrel\\Applications\\org.anodrel.test");
        assert_eq!(
            direct_version(root, &root.join("0.1.0")),
            Some(crate::PackageVersion::new(0, 1, 0))
        );
        assert!(direct_version(root, &root.join("nested").join("0.1.0")).is_none());
        assert!(direct_version(root, &root.join("0.01.0")).is_none());
    }
}
