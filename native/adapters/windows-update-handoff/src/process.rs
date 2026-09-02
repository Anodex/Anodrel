//! Opaque process ownership for a direct fixed-command elevated update.

use std::fmt;

use anodrel_windows_update_download::{
    UpdateCompletionError, VerifiedDownloadedInstaller, VerifiedUpdateInstallation,
    verify_current_update_selection,
};

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
    running: Option<RunningUpdate>,
}

struct RunningUpdate {
    process: UpdateProcessHandle,
    installer: VerifiedDownloadedInstaller,
}

impl ElevatedUpdateProcess {
    /// Waits for process completion while retaining the image for postcondition proof.
    pub fn wait(mut self) -> Result<CompletedElevatedUpdate, UpdateHandoffError> {
        let Some(mut running) = self.running.take() else {
            return Err(UpdateHandoffError::ProcessWaitFailed);
        };
        let succeeded = match running.process.wait() {
            Ok(succeeded) => succeeded,
            Err(error) => {
                running.installer.retain_for_recovery();
                return Err(error);
            }
        };
        let RunningUpdate { process, installer } = running;
        drop(process);
        let exit = if succeeded {
            ElevatedUpdateExit::Succeeded
        } else {
            ElevatedUpdateExit::Failed
        };
        Ok(CompletedElevatedUpdate { exit, installer })
    }
}

impl Drop for ElevatedUpdateProcess {
    fn drop(&mut self) {
        if let Some(running) = &mut self.running
            && running.process.completion_is_unconfirmed()
        {
            running.installer.retain_for_recovery();
        }
    }
}

/// One completed elevated update process retaining its candidate for verification.
pub struct CompletedElevatedUpdate {
    exit: ElevatedUpdateExit,
    installer: VerifiedDownloadedInstaller,
}

impl CompletedElevatedUpdate {
    /// Returns the installer process's conventional exit outcome.
    #[must_use]
    pub const fn exit(&self) -> ElevatedUpdateExit {
        self.exit
    }

    /// Proves the current fixed policy selected this candidate after a zero exit.
    ///
    /// A nonzero process exit is never treated as an update and performs no
    /// postcondition policy read. A successful result proves only current
    /// machine-policy selection, not restart or user-visible completion.
    pub fn verify_selection(self) -> Result<VerifiedUpdateInstallation, UpdateCompletionError> {
        match self.exit {
            ElevatedUpdateExit::Succeeded => verify_current_update_selection(&self.installer),
            ElevatedUpdateExit::Failed => Err(UpdateCompletionError::InstallerReportedFailure),
        }
    }
}

impl fmt::Debug for CompletedElevatedUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompletedElevatedUpdate(..)")
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
    Ok(ElevatedUpdateProcess {
        running: Some(RunningUpdate { process, installer }),
    })
}
