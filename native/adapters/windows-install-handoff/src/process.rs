//! Opaque process ownership for a direct fixed-command elevated first install.

use std::fmt;

use anodrel_windows_install_consent::ApprovedInitialInstall;
use anodrel_windows_installer::{
    InitialInstallCompletionError, VerifiedInitialInstallation, verify_current_initial_installation,
};

use crate::{InitialInstallHandoffError, raw::InitialInstallProcessHandle};

/// The conventional exit outcome of an elevated initial-installer process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElevatedInitialInstallExit {
    /// The installer process returned exit code zero.
    Succeeded,
    /// The installer process returned a nonzero exit code.
    Failed,
}

/// One UAC-approved elevated installer process and its original approval.
///
/// Call [`Self::wait`] away from a UI thread to observe ordinary process end.
/// An exit outcome is not proof that machine policy selected the release.
pub struct ElevatedInitialInstallProcess {
    running: Option<RunningInitialInstall>,
}

struct RunningInitialInstall {
    process: InitialInstallProcessHandle,
    approval: Box<ApprovedInitialInstall>,
}

impl ElevatedInitialInstallProcess {
    /// Waits for process completion while retaining the original approval.
    pub fn wait(mut self) -> Result<CompletedElevatedInitialInstall, InitialInstallHandoffError> {
        let Some(running) = self.running.take() else {
            return Err(InitialInstallHandoffError::ProcessWaitFailed);
        };
        let succeeded = running.process.wait()?;
        let RunningInitialInstall { process, approval } = running;
        drop(process);
        let exit = if succeeded {
            ElevatedInitialInstallExit::Succeeded
        } else {
            ElevatedInitialInstallExit::Failed
        };
        Ok(CompletedElevatedInitialInstall { exit, approval })
    }
}

/// One completed elevated first-install process retaining its original approval.
pub struct CompletedElevatedInitialInstall {
    exit: ElevatedInitialInstallExit,
    approval: Box<ApprovedInitialInstall>,
}

impl CompletedElevatedInitialInstall {
    /// Returns the installer process's conventional exit outcome.
    #[must_use]
    pub const fn exit(&self) -> ElevatedInitialInstallExit {
        self.exit
    }

    /// Proves the fixed policy selected the approved release after a zero exit.
    ///
    /// A nonzero process exit is never treated as installation and performs no
    /// postcondition policy read. A successful result proves only current
    /// machine-policy selection, not restart or user-visible completion.
    pub fn verify_installation(
        self,
    ) -> Result<VerifiedInitialInstallation, InitialInstallCompletionError> {
        match self.exit {
            ElevatedInitialInstallExit::Succeeded => {
                let prepared = (*self.approval).into_prepared();
                verify_current_initial_installation(&prepared)
            }
            ElevatedInitialInstallExit::Failed => {
                Err(InitialInstallCompletionError::InstallerReportedFailure)
            }
        }
    }
}

impl fmt::Debug for CompletedElevatedInitialInstall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CompletedElevatedInitialInstall(..)")
    }
}

impl fmt::Debug for ElevatedInitialInstallProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ElevatedInitialInstallProcess(..)")
    }
}

/// Starts Windows UAC only for one native-approved current installer release.
///
/// Windows receives the explicit `runas` verb and one fixed `install` argument.
/// This function performs no confirmation, download, policy read, progress UI,
/// restart, installer verification, or installation. User cancellation returns
/// [`InitialInstallHandoffError::UserDeclined`].
pub fn begin_elevated_initial_install(
    approval: Box<ApprovedInitialInstall>,
) -> Result<ElevatedInitialInstallProcess, InitialInstallHandoffError> {
    let process = InitialInstallProcessHandle::launch_current()?;
    Ok(ElevatedInitialInstallProcess {
        running: Some(RunningInitialInstall { process, approval }),
    })
}
