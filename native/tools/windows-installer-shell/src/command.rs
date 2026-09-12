//! Fixed command parsing and operator reporting for the Windows installer shell.

use std::fs;

use anodrel_windows_installer::{
    InstallCurrentError, MAX_RELEASE_MANIFEST_BYTES, ReleaseManifest,
    begin_current_uninstall_cleanup, install_current_signed_release, retire_current_cleanup_cache,
    rollback_current_signed_release, update_current_signed_release, verify_current_signed_release,
};

use crate::registered_uninstall;
use crate::{elevation::require_elevation, initial_install, install_exit};

const USAGE: &str = concat!(
    "usage:\n",
    "  anodrel-windows-installer\n",
    "  anodrel-windows-installer verify\n",
    "  anodrel-windows-installer install\n",
    "  anodrel-windows-installer update\n",
    "  anodrel-windows-installer rollback\n",
    "  anodrel-windows-installer remove\n",
    "  anodrel-windows-installer uninstall\n",
    "  anodrel-windows-installer cleanup-cache\n",
    "  anodrel-windows-installer validate-manifest <release-manifest.json>\n",
    "\n",
    "No-argument invocation starts the fixed signed initial-install flow. Named install,\n",
    "update, rollback, and uninstall commands require an elevated shell and accept no\n",
    "target or policy arguments. remove is the fixed native Apps & features route.\n",
    "validate-manifest is development-only and does not install anything.",
);

/// One fixed command accepted by the installer executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Command {
    InitialInstall,
    Verify,
    Install,
    Update,
    Rollback,
    Remove,
    Uninstall,
    Cleanup,
    CleanupCache,
    ValidateManifest(String),
}

/// A closed console error with a conventional process exit code.
pub(super) struct CommandError {
    message: String,
    exit_code: u8,
}

impl CommandError {
    /// Creates the conventional failure outcome for non-install commands.
    pub(super) fn general(error: impl std::fmt::Display) -> Self {
        Self {
            message: error.to_string(),
            exit_code: 1,
        }
    }

    /// Creates the fixed child outcome for an installation transaction failure.
    fn install(error: InstallCurrentError) -> Self {
        Self {
            exit_code: install_exit::code_for(&error),
            message: display_error(error),
        }
    }

    /// Returns the conventional process exit code.
    pub(super) const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// Parses only the supported fixed command shapes.
pub(super) fn parse(arguments: &[String]) -> Result<Command, String> {
    match arguments {
        [] => Ok(Command::InitialInstall),
        [command] if command == "verify" => Ok(Command::Verify),
        [command] if command == "install" => Ok(Command::Install),
        [command] if command == "update" => Ok(Command::Update),
        [command] if command == "rollback" => Ok(Command::Rollback),
        [command] if command == "remove" => Ok(Command::Remove),
        [command] if command == "uninstall" => Ok(Command::Uninstall),
        [command] if command == "cleanup" => Ok(Command::Cleanup),
        [command] if command == "cleanup-cache" => Ok(Command::CleanupCache),
        [command, path] if command == "validate-manifest" => {
            Ok(Command::ValidateManifest(path.clone()))
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// Performs one parsed command without exposing machine-owned operation inputs.
pub(super) fn execute(command: Command) -> Result<String, CommandError> {
    match command {
        Command::InitialInstall => initial_install::run().map_err(CommandError::general),
        Command::Verify => verify().map_err(CommandError::general),
        Command::Install => {
            require_elevation().map_err(CommandError::general)?;
            install().map_err(CommandError::install)
        }
        Command::Update => elevated(update).map_err(CommandError::general),
        Command::Rollback => elevated(rollback).map_err(CommandError::general),
        Command::Remove => registered_uninstall::run().map_err(CommandError::general),
        Command::Uninstall => elevated(uninstall).map_err(CommandError::general),
        Command::Cleanup => elevated(crate::cleanup_mode::run).map_err(CommandError::general),
        Command::CleanupCache => elevated(|| {
            retire_current_cleanup_cache().map_err(display_error)?;
            Ok("Exited Anodrel cleanup helpers retired.".to_owned())
        })
        .map_err(CommandError::general),
        Command::ValidateManifest(path) => validate_manifest(&path).map_err(CommandError::general),
    }
}

fn verify() -> Result<String, String> {
    let release = verify_current_signed_release().map_err(display_error)?;
    let manifest = release.release().manifest();
    let version = manifest.package_version();
    Ok(format!(
        "Current signed Anodrel release verified for {} version {}.{}.{}.",
        manifest.application_id(),
        version.major(),
        version.minor(),
        version.patch()
    ))
}

fn install() -> Result<String, InstallCurrentError> {
    install_current_signed_release()?;
    Ok("Current signed Anodrel release installed.".to_owned())
}

fn update() -> Result<String, String> {
    update_current_signed_release().map_err(display_error)?;
    Ok("Current signed Anodrel release updated.".to_owned())
}

fn rollback() -> Result<String, String> {
    rollback_current_signed_release().map_err(display_error)?;
    Ok("Current signed Anodrel release rolled back to its retained prior policy.".to_owned())
}

fn uninstall() -> Result<String, String> {
    begin_current_uninstall_cleanup().map_err(display_error)?;
    Ok(
        "Anodrel removal accepted; package cleanup finishes after this uninstaller exits."
            .to_owned(),
    )
}

fn elevated(action: fn() -> Result<String, String>) -> Result<String, String> {
    require_elevation().map_err(display_error)?;
    action()
}

fn validate_manifest(path: &str) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|_| "the release manifest could not be read")?;
    if !metadata.is_file() || metadata.len() > MAX_RELEASE_MANIFEST_BYTES as u64 {
        return Err("the release manifest could not be read".to_owned());
    }
    let manifest = fs::read_to_string(path)
        .map_err(|_| "the release manifest could not be read".to_owned())?;
    let release = ReleaseManifest::parse(&manifest).map_err(display_error)?;
    let version = release.package_version();
    Ok(format!(
        "Release manifest valid for {} version {}.{}.{}.",
        release.application_id(),
        version.major(),
        version.minor(),
        version.patch(),
    ))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{Command, USAGE, parse};

    #[test]
    fn commands_are_fixed_and_do_not_accept_machine_target_arguments() {
        assert_eq!(parse(&arguments(&[])), Ok(Command::InitialInstall));
        assert_eq!(parse(&arguments(&["verify"])), Ok(Command::Verify));
        assert_eq!(parse(&arguments(&["install"])), Ok(Command::Install));
        assert_eq!(parse(&arguments(&["update"])), Ok(Command::Update));
        assert_eq!(parse(&arguments(&["rollback"])), Ok(Command::Rollback));
        assert_eq!(parse(&arguments(&["remove"])), Ok(Command::Remove));
        assert_eq!(parse(&arguments(&["uninstall"])), Ok(Command::Uninstall));
        assert_eq!(parse(&arguments(&["cleanup"])), Ok(Command::Cleanup));
        assert_eq!(
            parse(&arguments(&["cleanup-cache"])),
            Ok(Command::CleanupCache)
        );
        assert_eq!(
            parse(&arguments(&["validate-manifest", "release.json"])),
            Ok(Command::ValidateManifest("release.json".to_owned()))
        );
        for invalid in [
            &["install", "C:\\target"][..],
            &["update", "--url", "https://example.test"][..],
            &["rollback", "1.2.3"][..],
            &["uninstall", "org.example.product"][..],
            &["cleanup", "C:\\target"][..],
            &["cleanup-cache", "org.example.product"][..],
            &["verify", "--registry"][..],
            &["--package", "release.bin"][..],
        ] {
            assert_eq!(parse(&arguments(invalid)), Err(USAGE.to_owned()));
        }
        assert!(USAGE.contains("No-argument invocation starts"));
    }

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }
}
