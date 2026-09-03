#![deny(unsafe_op_in_unsafe_fn)]

//! Locked, verified, and tracked Windows application launch.
//!
//! This host-only service reads one machine policy record, locks the exact
//! executable, revalidates its bytes, checks its embedded Authenticode signer,
//! and then delegates one-use bootstrap delivery to the direct process launcher.
//! It accepts no application arguments, shell command, environment override,
//! policy source, restart behavior, or public protocol request.

mod raw;

use std::{fmt, fs, io, path::Path};

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
    let verified = verify_locked_executable(application_id)?;
    let program = verified
        .path
        .to_str()
        .ok_or(LaunchError::InvalidExecutablePath)?;
    let command = BootstrapCommand::new(program).map_err(LaunchError::Command)?;
    let process =
        anodrel_windows_bootstrap::launch(&command, invitation).map_err(LaunchError::Bootstrap)?;
    // The lock is released only after CreateProcessW has returned, so the
    // process image cannot be replaced between verification and creation.
    drop(verified);

    Ok(TrackedApplication { process })
}

/// Runs the launch verification sequence without creating a process.
///
/// This exists so a host surface can decide whether a registered application is
/// currently launchable before it offers a launch action. It performs the same
/// machine-policy read, locked digest revalidation, Authenticode evaluation, and
/// publisher comparison as a launch, then releases the lock. It creates no
/// process, pipe, bootstrap material, or session, and returns no path,
/// certificate, digest, or native error.
///
/// A successful result describes this moment only. Every launch re-runs the
/// full sequence; an earlier verification never authorizes a later executable.
/// Call this from a worker, never the Win32 UI thread.
pub fn verify_registered_application(application_id: &str) -> Result<(), LaunchError> {
    verify_locked_executable(application_id).map(|_| ())
}

/// Verifies that the current host process is the selected product launcher.
///
/// This must run before the host creates a product window. It compares the
/// caller's canonical executable path with selected policy, then locks,
/// rehashes, and Authenticode-verifies that launcher. It does not start a
/// child, create a bootstrap record, or expose a process path to an
/// application.
pub fn verify_registered_product_launcher(
    application_id: &str,
    current_process: &Path,
) -> Result<(), LaunchError> {
    let installed = load_installed_application(application_id).map_err(LaunchError::Policy)?;
    let launcher_path = installed
        .product_launcher_path()
        .ok_or(LaunchError::ProductLauncherUnavailable)?;
    let current_path = fs::canonicalize(current_process).map_err(LaunchError::Io)?;
    if current_path != launcher_path {
        return Err(LaunchError::ProductLauncherMismatch);
    }
    let mut launcher = raw::lock_executable(launcher_path).map_err(LaunchError::Io)?;
    let locked_path = launcher.path().to_path_buf();
    installed
        .revalidate_product_launcher(&locked_path, &mut launcher)
        .map_err(LaunchError::Record)?;
    let signer = verify_embedded_signature(&locked_path).map_err(LaunchError::Signature)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(LaunchError::PublisherMismatch);
    }
    Ok(())
}

/// One executable that passed every pre-launch check, with its lock still held.
struct VerifiedExecutable {
    path: std::path::PathBuf,
    _lock: raw::LockedExecutable,
}

fn verify_locked_executable(application_id: &str) -> Result<VerifiedExecutable, LaunchError> {
    let installed = load_installed_application(application_id).map_err(LaunchError::Policy)?;
    let mut executable =
        raw::lock_executable(installed.executable_path()).map_err(LaunchError::Io)?;
    let path = executable.path().to_path_buf();
    installed
        .revalidate_executable(&path, &mut executable)
        .map_err(LaunchError::Record)?;

    let signer = verify_embedded_signature(&path).map_err(LaunchError::Signature)?;
    if !installed.matches_publisher(signer.as_bytes()) {
        return Err(LaunchError::PublisherMismatch);
    }
    Ok(VerifiedExecutable {
        path,
        _lock: executable,
    })
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
    ProductLauncherUnavailable,
    ProductLauncherMismatch,
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
            Self::ProductLauncherUnavailable => {
                "registered application does not declare a product launcher"
            }
            Self::ProductLauncherMismatch => {
                "current product launcher does not match registered policy"
            }
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
            Self::PublisherMismatch
            | Self::ProductLauncherUnavailable
            | Self::ProductLauncherMismatch
            | Self::InvalidExecutablePath => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use anodrel_windows_policy::PolicyStoreError;

    use super::{LaunchError, verify_registered_application, verify_registered_product_launcher};

    #[test]
    fn verification_rejects_an_invalid_identity_before_reading_machine_policy() {
        assert!(matches!(
            verify_registered_application("org.anodrel/escape"),
            Err(LaunchError::Policy(PolicyStoreError::InvalidApplicationId))
        ));
    }

    #[test]
    fn verification_fails_closed_on_an_unprovisioned_machine_record() {
        // A host surface must treat this as "no launch action", never as a
        // reason to try launching anyway.
        assert!(matches!(
            verify_registered_application("org.anodrel.launch-verify-unprovisioned-test"),
            Err(LaunchError::Policy(PolicyStoreError::RecordNotFound))
        ));
    }

    #[test]
    fn launch_failure_categories_stay_free_of_native_detail() {
        // The displayed category is what a host may surface; it must never
        // name a path, certificate, digest, or Windows status.
        let message = LaunchError::PublisherMismatch.to_string();
        assert_eq!(message, "registered application publisher is not approved");
    }

    #[test]
    fn product_launcher_verification_rejects_an_invalid_identity_before_reading_policy() {
        assert!(matches!(
            verify_registered_product_launcher("org.anodrel/escape", std::path::Path::new("x")),
            Err(LaunchError::Policy(PolicyStoreError::InvalidApplicationId))
        ));
    }
}
