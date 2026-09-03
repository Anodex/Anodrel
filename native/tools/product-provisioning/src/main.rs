//! Controlled provisioning for the development Windows product fixture.
//!
//! This is a development helper, not part of the native host. It is the only
//! component in the repository that writes machine policy, and it can write
//! exactly one value: the `record` for `org.anodrel.product-fixture` under the
//! documented `HKEY_LOCAL_MACHINE` policy key. It cannot name a hive, key path,
//! value name, application ID, or capability.
//!
//! `provision` and `remove` change machine state and need an elevated shell.
//! `stage` and `verify` do not. See `docs/PRODUCT_FIXTURE.md`.

#![deny(unsafe_op_in_unsafe_fn)]

mod fixture;
mod package;
mod record;
mod registry;

use std::{env, process::ExitCode};

const USAGE: &str = concat!(
    "usage: anodrel-product-provisioning <command>\n",
    "  stage <package-root>       write the fixture manifest and content\n",
    "  provision <package-root>   verify signed child and launcher images, then write machine policy\n",
    "  verify                     report whether the machine record currently validates\n",
    "  remove                     delete the fixture machine-policy key",
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, package_root] if command == "stage" => stage(package_root),
        [command, package_root] if command == "provision" => provision(package_root),
        [command] if command == "verify" => verify(),
        [command] if command == "remove" => remove(),
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

fn stage(package_root: &str) -> Result<String, String> {
    let root = std::path::Path::new(package_root);
    package::stage(root).map_err(|_| {
        "the fixture package could not be staged; check the target directory".to_owned()
    })?;
    Ok(format!(
        "Staged the {} package manifest and content. Copy signed child and launcher images to bin\\{} and bin\\{} next.",
        fixture::APPLICATION_ID,
        fixture::EXECUTABLE_FILE_NAME,
        fixture::LAUNCHER_FILE_NAME,
    ))
}

fn provision(package_root: &str) -> Result<String, String> {
    let root = record::canonical_package_root(package_root)
        .map_err(|_| "the fixture package root is not a readable directory".to_owned())?;
    let executable = package::executable(&root).map_err(|_| {
        format!(
            "no staged executable was found at bin\\{}",
            fixture::EXECUTABLE_FILE_NAME
        )
    })?;
    let launcher = package::launcher(&root).map_err(|_| {
        format!(
            "no staged product launcher was found at bin\\{}",
            fixture::LAUNCHER_FILE_NAME
        )
    })?;

    // Composition performs the digest, Authenticode, and record checks, so a
    // record that cannot pass the host's own parser is never written.
    let composed = record::compose(&root, &executable, &launcher)
        .map_err(|error| format!("{error}. Nothing changed."))?;
    registry::write_record(fixture::APPLICATION_ID, &composed)
        .map_err(|error| format!("{error}. Nothing changed."))?;

    verify().map(|report| {
        format!(
            "Provisioned machine policy for {}.\n{report}",
            fixture::APPLICATION_ID
        )
    })
}

/// Reports whether the machine record currently validates, using the host's own
/// read-only policy adapter rather than a second implementation.
fn verify() -> Result<String, String> {
    match anodrel_windows_policy::load_installed_application(fixture::APPLICATION_ID) {
        Ok(_) => Ok(format!(
            "The machine record for {} validates against its staged package and executable.",
            fixture::APPLICATION_ID
        )),
        Err(error) => Err(format!(
            "The machine record for {} does not validate: {error}",
            fixture::APPLICATION_ID
        )),
    }
}

fn remove() -> Result<String, String> {
    registry::remove_record(fixture::APPLICATION_ID).map_err(|error| error.to_string())?;
    Ok(format!(
        "Removed the machine-policy key for {}. The staged package and certificate are removed separately.",
        fixture::APPLICATION_ID
    ))
}

#[cfg(test)]
mod tests {
    use super::{USAGE, fixture, verify};

    #[test]
    fn usage_names_every_supported_command_and_nothing_else() {
        for command in ["stage", "provision", "verify", "remove"] {
            assert!(USAGE.contains(command), "usage omits {command}");
        }
        // There is deliberately no command that accepts a registry path, value
        // name, application ID, or capability.
        for absent in ["--key", "--value", "--application-id", "--capability"] {
            assert!(!USAGE.contains(absent), "usage exposes {absent}");
        }
    }

    #[test]
    fn verification_fails_closed_and_names_only_the_fixture_identity() {
        // On an unprovisioned machine this is the expected result. On a
        // provisioned one it succeeds; either way the message must carry only
        // the fixture identity and a safe category.
        let message = match verify() {
            Ok(message) | Err(message) => message,
        };
        assert!(message.contains(fixture::APPLICATION_ID));
        assert!(!message.contains(":\\"), "a message must not carry a path");
    }
}
