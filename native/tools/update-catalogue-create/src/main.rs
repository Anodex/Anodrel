//! Command-line entry point for locked-image update catalogue authoring.

use std::{env, path::Path, process::ExitCode};

use anodrel_update_catalogue_create::create_update_catalogue;

const USAGE: &str = concat!(
    "usage: anodrel-update-catalogue-create create <signed-installer.exe> <https-host> <https-port> <installer-path> <new-catalogue.json>\n",
    "\n",
    "Derives one fresh strict unsigned catalogue from a locked signed installer.\n",
    "It never signs, publishes, installs, launches, or retrieves an update."
);

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = match arguments.as_slice() {
        [command, image, host, port, installer_path, output] if command == "create" => port
            .parse()
            .map_err(|_| "the HTTPS port is invalid".to_owned())
            .and_then(|port| {
                create_update_catalogue(
                    Path::new(image),
                    host,
                    port,
                    installer_path,
                    Path::new(output),
                )
                .map_err(|error| error.to_string())
            }),
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(()) => {
            println!("Created a checked unsigned update catalogue.");
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
    fn usage_exposes_no_signing_or_machine_operation() {
        for absent in ["install", "update", "sign", "--certificate", "--registry"] {
            assert!(
                !USAGE.split_whitespace().any(|word| word == absent),
                "usage exposes {absent}"
            );
        }
    }
}
