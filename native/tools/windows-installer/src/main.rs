//! Read-only validation entry point for the owned Windows installer.

use std::{env, fs, process::ExitCode};

use anodrel_windows_installer::{MAX_RELEASE_MANIFEST_BYTES, ReleaseManifest};

const USAGE: &str = concat!(
    "usage: anodrel-windows-installer validate <embedded-release-manifest.json>\n",
    "\n",
    "This foundation validates a release manifest only; it does not install,\n",
    "modify machine policy, write a package directory, or accept a target path.",
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, path] if command == "validate" => validate(path),
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn validate(path: &str) -> Result<String, String> {
    let metadata =
        fs::metadata(path).map_err(|_| "the release manifest could not be read".to_owned())?;
    if !metadata.is_file() || metadata.len() > MAX_RELEASE_MANIFEST_BYTES as u64 {
        return Err("the release manifest could not be read".to_owned());
    }
    let manifest = fs::read_to_string(path)
        .map_err(|_| "the release manifest could not be read".to_owned())?;
    let release = ReleaseManifest::parse(&manifest).map_err(|error| error.to_string())?;
    let version = release.package_version();
    Ok(format!(
        "Release manifest valid for {} version {}.{}.{}.",
        release.application_id(),
        version.major(),
        version.minor(),
        version.patch(),
    ))
}

#[cfg(test)]
mod tests {
    use super::USAGE;

    #[test]
    fn usage_exposes_only_the_read_only_validation_slice() {
        assert!(USAGE.contains("validate"));
        for absent in [
            "install",
            "uninstall",
            "--root",
            "--registry",
            "--capability",
        ] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
