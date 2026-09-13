//! Signed no-restart uninstall coordination and bounded cleanup-cache retirement.

use crate::{
    remove_current_apps_features, remove_current_product_shortcut,
    remove_verified_uninstall_policy, verify_current_signed_release,
    verify_current_uninstall_target,
};
use anodrel_windows_policy::{PolicyStoreError, load_installed_application};
use std::{
    fmt,
    io::Write,
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

mod cache;
mod channel;
mod prior;
mod stage;
#[cfg(test)]
mod tests;

pub(crate) use cache::retire as retire_cache;

const READY: [u8; 4] = *b"ACR1";
const COMMIT: [u8; 4] = *b"ACC1";
const ACCEPTED: [u8; 4] = *b"ACA1";
const TIMEOUT: Duration = Duration::from_secs(30);

/// A fixed cleanup stage failed. The error carries no machine paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupError {
    /// Current installer or helper did not match the signed installed release.
    Verification,
    /// The protected helper cache could not be prepared or reclaimed.
    Cache,
    /// The private handshake failed or timed out before commit.
    Handoff,
    /// Registration or selected-policy removal failed.
    Policy,
    /// Package files remain in use or could not be removed.
    Package,
}
impl fmt::Display for CleanupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Verification => "the signed cleanup image or selected package did not verify",
            Self::Cache => "the protected cleanup cache could not be prepared or reclaimed",
            Self::Handoff => "the private cleanup handoff failed or timed out",
            Self::Policy => "the cleanup transaction did not remove registration and policy",
            Self::Package => {
                "package cleanup is incomplete; close the application, then run the prepared signed installer's cleanup-cache command from an elevated shell"
            }
        })
    }
}
impl std::error::Error for CleanupError {}

/// Starts only a verified copy of the selected installed uninstaller.
///
/// Returns after the child has removed policy. Final file removal waits for
/// this process and the normal-user removal window to exit; the helper reports
/// its final result separately. No caller can choose an image or target.
pub fn begin_current_uninstall_cleanup() -> Result<(), CleanupError> {
    let target = verify_current_uninstall_target().map_err(|_| CleanupError::Verification)?;
    let release = verify_current_signed_release().map_err(|_| CleanupError::Verification)?;
    let maintenance = crate::maintenance::MaintenanceLock::acquire(
        target.package_root().parent().ok_or(CleanupError::Cache)?,
    )
    .map_err(|_| CleanupError::Cache)?;
    let target = verify_current_uninstall_target().map_err(|_| CleanupError::Verification)?;
    let staged = stage::prepare(&target, release.release().manifest())?;
    // The staged image remains locked until the child takes the shared
    // maintenance lock and proves the still-selected package independently.
    drop(maintenance);
    let mut pending = PendingChild(Some(
        Command::new(staged.image())
            .arg("cleanup")
            .current_dir(staged.directory())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|_| CleanupError::Handoff)?,
    ));
    let child = pending.0.as_mut().ok_or(CleanupError::Handoff)?;
    let input = child.stdin.as_mut().ok_or(CleanupError::Handoff)?;
    // The stream carries only fixed control frames. Identity and paths are
    // independently derived by the helper from its signed release and policy.
    channel::read(
        child.stdout.as_mut().ok_or(CleanupError::Handoff)?,
        READY,
        TIMEOUT,
    )?;
    input
        .write_all(&COMMIT)
        .map_err(|_| CleanupError::Handoff)?;
    // After COMMIT the helper owns the transaction, even if this parent dies.
    let mut child = pending.0.take().ok_or(CleanupError::Handoff)?;
    drop(child.stdin.take());
    channel::read(
        child.stdout.as_mut().ok_or(CleanupError::Handoff)?,
        ACCEPTED,
        TIMEOUT,
    )?;
    Ok(())
}

/// Runs the private helper after elevation and current-image checks.
///
/// The helper accepts no target, PID, policy, or path argument. A failed or
/// absent private commit leaves policy and the package untouched.
pub fn run_current_uninstall_cleanup() -> Result<(), CleanupError> {
    let release = verify_current_signed_release().map_err(|_| CleanupError::Verification)?;
    let root = crate::machine_root::existing_machine_application_root(
        release.release().manifest().application_id(),
    )
    .map_err(|_| CleanupError::Verification)?;
    let _maintenance = crate::maintenance::MaintenanceLock::acquire(root.path())
        .map_err(|_| CleanupError::Cache)?;
    let target = stage::verify_helper()?;
    channel::write_stdout(READY)?;
    channel::read_stdin(COMMIT, TIMEOUT)?;
    remove_current_product_shortcut().map_err(|_| CleanupError::Policy)?;
    remove_current_apps_features().map_err(|_| CleanupError::Policy)?;
    let current = std::env::current_exe().map_err(|_| CleanupError::Cache)?;
    let marker = cache::mark_committed(current.parent().ok_or(CleanupError::Cache)?)?;
    let removed = remove_verified_uninstall_policy(target).map_err(|_| CleanupError::Policy)?;
    let current_version = release.release().manifest().package_version();
    let publisher = *release.release().manifest().publisher_fingerprint();
    // Failure to notify a dead parent must not strand a committed cleanup.
    let _ = channel::write_stdout(ACCEPTED);
    let deadline = Instant::now() + TIMEOUT;
    loop {
        // A later install may select another version. Never proceed if any
        // selected policy reappears, including an invalid record.
        if !matches!(
            load_installed_application(removed.application_id()),
            Err(PolicyStoreError::RecordNotFound)
        ) {
            return Err(CleanupError::Policy);
        }
        match crate::recovery::raw::remove_normal_tree(removed.package_root()) {
            Ok(()) => {
                prior::remove_verified_prior(
                    root.path(),
                    removed.application_id(),
                    publisher,
                    current_version,
                )?;
                std::fs::remove_file(marker).map_err(|_| CleanupError::Cache)?;
                return Ok(());
            }
            Err(crate::RecoveryCleanupError::ReparsePointRefused) => {
                return Err(CleanupError::Package);
            }
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Err(_) => return Err(CleanupError::Package),
        }
    }
}

/// Retires exited helper copies for this signed release's application/publisher.
/// Resumes committed package cleanup only with absent policy and signed cache
/// proof. Never removes policy, user data, certificates, or a running helper.
pub fn retire_current_cleanup_cache() -> Result<(), CleanupError> {
    let release = verify_current_signed_release().map_err(|_| CleanupError::Verification)?;
    let root = crate::machine_root::existing_machine_application_root(
        release.release().manifest().application_id(),
    )
    .map_err(|_| CleanupError::Cache)?;
    let _maintenance = crate::maintenance::MaintenanceLock::acquire(root.path())
        .map_err(|_| CleanupError::Cache)?;
    cache::retire(root.path(), release.release().manifest())?;
    match load_installed_application(release.release().manifest().application_id()) {
        Err(PolicyStoreError::RecordNotFound) => prior::remove_verified_prior(
            root.path(),
            release.release().manifest().application_id(),
            *release.release().manifest().publisher_fingerprint(),
            release.release().manifest().package_version(),
        ),
        Ok(_) => Ok(()),
        Err(_) => Err(CleanupError::Policy),
    }
}

struct PendingChild(Option<Child>);
impl Drop for PendingChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
