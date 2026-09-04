//! Fixed command parsing and operator reporting for the Windows installer shell.

use std::fs;

use anodrel_windows_installer::{
    MAX_RELEASE_MANIFEST_BYTES, ReleaseManifest, install_current_signed_release,
    remove_current_apps_features, remove_current_product_shortcut, remove_policy_removed_package,
    remove_verified_uninstall_policy, rollback_current_signed_release,
    update_current_signed_release, verify_current_signed_release, verify_current_uninstall_target,
};

use crate::registered_uninstall;
use crate::{elevation::require_elevation, initial_install};

const USAGE: &str = concat!(
    "usage:\n",
    "  anodrel-windows-installer\n",
    "  anodrel-windows-installer verify\n",
    "  anodrel-windows-installer install\n",
    "  anodrel-windows-installer update\n",
    "  anodrel-windows-installer rollback\n",
    "  anodrel-windows-installer remove\n",
    "  anodrel-windows-installer uninstall\n",
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
    ValidateManifest(String),
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
        [command, path] if command == "validate-manifest" => {
            Ok(Command::ValidateManifest(path.clone()))
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// Performs one parsed command without exposing machine-owned operation inputs.
pub(super) fn execute(command: Command) -> Result<String, String> {
    match command {
        Command::InitialInstall => initial_install::run(),
        Command::Verify => verify(),
        Command::Install => elevated(install),
        Command::Update => elevated(update),
        Command::Rollback => elevated(rollback),
        Command::Remove => registered_uninstall::run(),
        Command::Uninstall => elevated(uninstall),
        Command::ValidateManifest(path) => validate_manifest(&path),
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

fn install() -> Result<String, String> {
    install_current_signed_release().map_err(display_error)?;
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
    let target = verify_current_uninstall_target().map_err(display_error)?;
    remove_current_product_shortcut().map_err(display_error)?;
    remove_current_apps_features().map_err(display_error)?;
    let policy_removed = remove_verified_uninstall_policy(target).map_err(display_error)?;
    remove_policy_removed_package(policy_removed).map_err(display_error)?;
    Ok("Current signed Anodrel release uninstalled.".to_owned())
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
        assert_eq!(
            parse(&arguments(&["validate-manifest", "release.json"])),
            Ok(Command::ValidateManifest("release.json".to_owned()))
        );
        for invalid in [
            &["install", "C:\\target"][..],
            &["update", "--url", "https://example.test"][..],
            &["rollback", "1.2.3"][..],
            &["uninstall", "org.example.product"][..],
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
