//! Selected product-launcher validation kept separate from the application child.

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use crate::sha256;

use super::{InstalledApplicationError, MAX_EXECUTABLE_BYTES, canonical_executable_path};

/// One contained launcher that a selected Windows product may use as its host.
pub(super) struct ProductLauncher {
    path: PathBuf,
    digest: [u8; 32],
}

impl ProductLauncher {
    /// Validates one distinct launcher declaration against a checked package.
    pub(super) fn from_record(
        package_root: &Path,
        application_executable: &Path,
        declared_path: &str,
        digest: [u8; 32],
    ) -> Result<Self, InstalledApplicationError> {
        let path = canonical_executable_path(package_root, declared_path)?;
        if path == application_executable {
            return Err(InstalledApplicationError::ProductLauncherMatchesApplication);
        }
        Ok(Self { path, digest })
    }

    /// Returns the private canonical launcher path for host composition only.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Rechecks this launcher through a caller-held operating-system lock.
    pub(super) fn revalidate<R: Read>(
        &self,
        path: &Path,
        reader: &mut R,
    ) -> Result<(), InstalledApplicationError> {
        let canonical_path = std::fs::canonicalize(path).map_err(InstalledApplicationError::Io)?;
        if canonical_path != self.path {
            return Err(InstalledApplicationError::ProductLauncherPathChanged);
        }
        let (actual_digest, _) = sha256::digest_reader_limited(reader, MAX_EXECUTABLE_BYTES)
            .map_err(InstalledApplicationError::Io)?
            .ok_or(InstalledApplicationError::ExecutableTooLarge)?;
        if actual_digest == self.digest {
            Ok(())
        } else {
            Err(InstalledApplicationError::ProductLauncherDigestMismatch)
        }
    }
}

impl std::fmt::Debug for ProductLauncher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProductLauncher(..)")
    }
}
