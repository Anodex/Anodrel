//! Command-line entry point for owned signed update-catalogue authoring.

use std::{env, path::Path, process::ExitCode};

use anodrel_update_catalogue_sign::sign_catalogue_file;

const USAGE: &str = concat!(
    "usage: anodrel-update-catalogue-sign sign <catalogue.json> <certificate-sha256> <new-catalogue.p7s>\n",
    "\n",
    "Creates one fresh attached-CMS catalogue with one exact current-user certificate.\n",
    "It never overwrites, retrieves, installs, launches, elevates, or changes trust."
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, catalogue, fingerprint, output] if command == "sign" => {
            sign_catalogue_file(Path::new(catalogue), fingerprint, Path::new(output))
                .map_err(|error| error.to_string())
        }
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => {
            println!("Created a checked signed update catalogue.");
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
    fn usage_exposes_only_fresh_catalogue_signing() {
        assert!(USAGE.contains("sign"));
        for absent in ["install", "update", "download", "--overwrite", "--registry"] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
