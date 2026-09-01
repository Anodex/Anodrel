//! Command-line entry point for owned release-bundle authoring.

use std::{env, path::Path, process::ExitCode};

use anodrel_release_bundle_tool::create_release_bundle;

const USAGE: &str = concat!(
    "usage: anodrel-release-bundle-tool create <source-directory> <new-bundle>\n",
    "\n",
    "Creates one new synchronized owned release bundle from a normal directory.\n",
    "It never overwrites, embeds, signs, installs, launches, or downloads."
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, source, output] if command == "create" => create(source, output),
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => {
            println!("Created a verified owned release bundle.");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn create(source: &str, output: &str) -> Result<(), String> {
    create_release_bundle(Path::new(source), Path::new(output)).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::USAGE;

    #[test]
    fn usage_exposes_only_fresh_bundle_authoring() {
        assert!(USAGE.contains("create"));
        for absent in ["install", "uninstall", "sign", "--overwrite", "--registry"] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
