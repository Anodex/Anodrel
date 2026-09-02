//! Opaque process ownership for a direct fixed-command elevated update.

use std::fmt;

use anodrel_windows_update_download::VerifiedDownloadedInstaller;

use crate::{UpdateHandoffError, raw::UpdateProcessHandle};

/// The conventional exit outcome of an elevated installer process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElevatedUpdateExit {
    /// The installer process returned exit code zero.
    Succeeded,
    /// The installer process returned a nonzero exit code.
    Failed,
}

/// One UAC-approved elevated update process and its still-locked source image.
///
/// Call [`Self::wait`] away from a UI thread to observe ordinary process end.
/// An exit outcome is not proof that machine policy selected a new release: the
/// installed process independently reports only its own bounded command result.
pub struct ElevatedUpdateProcess {
    process: UpdateProcessHandle,
    installer: VerifiedDownloadedInstaller,
}

impl ElevatedUpdateProcess {
    /// Waits for process completion and then releases the private image.
    pub fn wait(self) -> Result<ElevatedUpdateExit, UpdateHandoffError> {
        self.process.wait().map(|succeeded| {
            if succeeded {
                ElevatedUpdateExit::Succeeded
            } else {
                ElevatedUpdateExit::Failed
            }
        })
    }
}

impl Drop for ElevatedUpdateProcess {
    fn drop(&mut self) {
        if self.process.completion_is_unconfirmed() {
            self.installer.retain_for_recovery();
        }
    }
}

impl fmt::Debug for ElevatedUpdateProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ElevatedUpdateProcess(..)")
    }
}

/// Starts Windows UAC only for one locked exact installer image.
///
/// Windows receives the explicit `runas` verb and the one fixed `update`
/// argument. This function performs no update discovery, download, cache
/// selection, progress UI, restart, installer verification, or installation.
/// User cancellation returns [`UpdateHandoffError::UserDeclined`].
pub fn begin_elevated_update(
    mut installer: VerifiedDownloadedInstaller,
) -> Result<ElevatedUpdateProcess, UpdateHandoffError> {
    let process = match UpdateProcessHandle::launch(installer.path()) {
        Ok(process) => process,
        Err(error @ UpdateHandoffError::ProcessUnavailable) => {
            // Shell execution can begin through an unobservable route. Preserve
            // the image for owned recovery rather than risking a delete while a
            // process may still be opening it.
            installer.retain_for_recovery();
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    Ok(ElevatedUpdateProcess { process, installer })
}
