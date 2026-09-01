//! Command-line entry point for owned Windows release signing.

use std::{env, path::Path, process::ExitCode};

use anodrel_release_sign::sign_release_image;

const USAGE: &str = concat!(
    "usage: anodrel-release-sign sign <unsigned-release-image> <certificate-sha256> <new-signed-image>\n",
    "\n",
    "Creates one fresh signed release image from a checked Anodrel release image.\n",
    "It never overwrites, selects a certificate by subject, creates trust, timestamps, installs, or launches."
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, input, certificate, output] if command == "sign" => {
            sign_release_image(Path::new(input), certificate, Path::new(output))
                .map_err(|error| error.to_string())
        }
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => {
            println!("Created and verified a signed Anodrel release image.");
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
    fn usage_exposes_only_explicit_fresh_signing() {
        assert!(USAGE.contains("sign"));
        for absent in [
            "install",
            "uninstall",
            "timestamp",
            "--overwrite",
            "--registry",
        ] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
