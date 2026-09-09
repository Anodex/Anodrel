//! Fixed operator commands and no-argument flow for the owned Windows installer.

mod command;
mod elevation;
mod initial_install;
mod install_exit;
mod registered_uninstall;

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = command::parse(&arguments)
        .map_err(command::CommandError::general)
        .and_then(command::execute);
    match outcome {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
