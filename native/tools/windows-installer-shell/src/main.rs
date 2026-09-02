//! Fixed operator commands and no-argument flow for the owned Windows installer.

mod command;
mod elevation;
mod initial_install;

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let outcome = command::parse(&arguments).and_then(command::execute);
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
