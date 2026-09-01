//! Command-line entry point for owned release-manifest authoring.

use std::{env, path::Path, process::ExitCode};

use anodrel_release_manifest::create_release_manifest;

const USAGE: &str = concat!(
    "usage: anodrel-release-manifest create <release-plan.json> <bundle.bin> <new-manifest.json>\n",
    "\n",
    "Derives one fresh strict release manifest from a checked owned bundle.\n",
    "It never overwrites, extracts, signs, installs, launches, or downloads."
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, plan, bundle, output] if command == "create" => {
            create_release_manifest(Path::new(plan), Path::new(bundle), Path::new(output))
                .map_err(|error| error.to_string())
        }
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => {
            println!("Created a checked owned release manifest.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::USAGE;

    #[test]
    fn usage_exposes_only_fresh_manifest_authoring() {
        assert!(USAGE.contains("create"));
        for absent in ["install", "uninstall", "sign", "--overwrite", "--registry"] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
