#![deny(unsafe_op_in_unsafe_fn)]

//! Locked, verified, and tracked Windows application launch.
//!
//! This host-only service reads one machine policy record, locks the exact
//! executable, revalidates its bytes, checks its embedded Authenticode signer,
//! and then delegates one-use bootstrap delivery to the direct process launcher.
//! It accepts no application arguments, shell command, environment override,
//! policy source, restart behavior, or public protocol request.

mod raw;

use std::{fmt, io};

use anodrel_application::InstalledApplicationError;
use anodrel_bootstrap::BootstrapInvitation;
use anodrel_windows_bootstrap::{
    BootstrapCommand, BootstrapLaunchError, CommandError, LaunchedProcess,
};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use anodrel_windows_signature::{SignatureError, verify_embedded_signature};

const SHUTDOWN_EXIT_CODE: u32 = 0xA11D;

/// Launches the one executable registered for a host-selected application ID.
///
/// The supplied invitation is delivered only after machine policy, locked-file
/// digest validation, and Authenticode publisher validation succeed. Call this
/// from a worker, never the Win32 UI thread.
pub fn launch_registered_application(
    application_id: &str,
    invitation: &BootstrapInvitation,
) -> Result<TrackedApplication, LaunchError> {
    let installed = load_installed_application(application_id).map_err(LaunchError::Policy)?;
    let mut executable =
        raw::lock_executable(installed.executable_path()).map_err(LaunchError::Io)?;
    let executable_path = executable.path().to_path_buf();
    installed
        .revalidate_executable(&executable_path, &mut executable)
        .map_err(LaunchError::Record)?;

    let signer = verify_embedded_signature(&executable_path).map_err(LaunchError::Signature)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(LaunchError::PublisherMismatch);
    }

    let program = executable_path
        .to_str()
        .ok_or(LaunchError::InvalidExecutablePath)?;
    let command = BootstrapCommand::new(program).map_err(LaunchError::Command)?;
    let process =
        anodrel_windows_bootstrap::launch(&command, invitation).map_err(LaunchError::Bootstrap)?;
    drop(executable);

    Ok(TrackedApplication { process })
}

/// A launched child whose lifetime remains attached to the native host.
pub struct TrackedApplication {
    process: LaunchedProcess,
}

impl TrackedApplication {
    /// Waits for the child to exit and returns its exit code.
    pub fn wait_for_exit(&self, timeout_milliseconds: u32) -> io::Result<u32> {
        self.process.wait_for_exit(timeout_milliseconds)
    }

    /// Explicitly stops the child during controlled host shutdown.
    pub fn terminate(&self) -> io::Result<()> {
        self.process.terminate(SHUTDOWN_EXIT_CODE)
    }
}

impl Drop for TrackedApplication {
    fn drop(&mut self) {
        // A host must not orphan a child because a launch result went out of
        // scope during shutdown or an error path. An already exited child is
        // harmless; its Windows error is intentionally ignored here.
        let _ = self.terminate();
    }
}

impl fmt::Debug for TrackedApplication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TrackedApplication(..)")
    }
}

/// A safe category for a registered application launch failure.
#[derive(Debug)]
pub enum LaunchError {
    Policy(PolicyStoreError),
    Io(io::Error),
    Record(InstalledApplicationError),
    Signature(SignatureError),
    PublisherMismatch,
    InvalidExecutablePath,
    Command(CommandError),
    Bootstrap(BootstrapLaunchError),
}

impl fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Policy(_) => "registered application policy could not be validated",
            Self::Io(_) => "registered application executable could not be locked",
            Self::Record(_) => "registered application executable did not revalidate",
            Self::Signature(_) => "registered application signature did not verify",
            Self::PublisherMismatch => "registered application publisher is not approved",
            Self::InvalidExecutablePath => "registered application executable path is invalid",
            Self::Command(_) => "registered application launch command is invalid",
            Self::Bootstrap(_) => "registered application bootstrap launch failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Record(error) => Some(error),
            Self::Signature(error) => Some(error),
            Self::Command(error) => Some(error),
            Self::Bootstrap(error) => Some(error),
            Self::PublisherMismatch | Self::InvalidExecutablePath => None,
        }
    }
}
