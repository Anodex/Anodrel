//! Command-line entry point for owned pre-signing release-image assembly.

use std::{env, fs, path::Path, process::ExitCode};

use anodrel_release_image::embed_release_image;
use anodrel_windows_installer::{MAX_PAYLOAD_BYTES, MAX_RELEASE_MANIFEST_BYTES};

const USAGE: &str = concat!(
    "usage: anodrel-release-image embed <template.exe> <manifest.json> <bundle.bin> <new-output.exe>\n",
    "\n",
    "Creates a new unsigned resource-bearing image. It never overwrites an output\n",
    "or installs, signs, or launches an application.",
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, template, manifest, payload, output] if command == "embed" => {
            embed(template, manifest, payload, output)
        }
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

fn embed(template: &str, manifest: &str, payload: &str, output: &str) -> Result<String, String> {
    let manifest = read_limited(Path::new(manifest), MAX_RELEASE_MANIFEST_BYTES)?;
    let payload = read_limited(Path::new(payload), MAX_PAYLOAD_BYTES as usize)?;
    embed_release_image(Path::new(template), Path::new(output), &manifest, &payload)
        .map_err(|error| error.to_string())?;
    Ok(
        "Created a verified unsigned release image. Sign it before distribution or installation."
            .to_owned(),
    )
}

fn read_limited(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    let metadata =
        fs::metadata(path).map_err(|_| "the release input could not be read".to_owned())?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err("the release input could not be read".to_owned());
    }
    fs::read(path).map_err(|_| "the release input could not be read".to_owned())
}

#[cfg(test)]
mod tests {
    use super::USAGE;

    #[test]
    fn usage_exposes_only_fresh_unsigned_image_assembly() {
        assert!(USAGE.contains("embed"));
        for absent in ["install", "uninstall", "sign", "--overwrite", "--registry"] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
