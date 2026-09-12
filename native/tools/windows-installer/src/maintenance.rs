//! Process-scoped exclusion for fixed per-application maintenance transactions.

use crate::MachineRootError;
use std::{
    fs::{File, OpenOptions},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

/// The handle is deliberately non-inherited and released even on process exit.
pub(crate) struct MaintenanceLock {
    // The empty file remains; deleting its name could permit split locks.
    _file: File,
}

impl MaintenanceLock {
    pub(crate) fn acquire(root: &Path) -> Result<Self, MachineRootError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(0x0020_0000)
            .open(root.join(".anodrel-maintenance.lock"))
            .map_err(|_| MachineRootError::MaintenanceUnavailable)?;
        let info = file
            .metadata()
            .map_err(|_| MachineRootError::MaintenanceUnavailable)?;
        if !info.is_file() || info.file_attributes() & 0x400 != 0 {
            return Err(MachineRootError::MaintenanceUnavailable);
        }
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_transactions_fail_and_release_allows_immediate_reuse() {
        let dir = crate::test_support::TestDirectory::new("maintenance-lock");
        let first = MaintenanceLock::acquire(dir.path()).unwrap();
        assert!(MaintenanceLock::acquire(dir.path()).is_err());
        assert!(std::fs::remove_file(dir.path().join(".anodrel-maintenance.lock")).is_err());
        drop(first);
        assert!(MaintenanceLock::acquire(dir.path()).is_ok());
    }
}
