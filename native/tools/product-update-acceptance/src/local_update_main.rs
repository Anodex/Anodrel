//! Command-line entry point for local signed-update fixture acceptance.

use std::{env, process::ExitCode};

use anodrel_product_update_acceptance::run_local_update_fixture;

const USAGE: &str = "usage: anodrel-local-update-fixture-acceptance";

fn main() -> ExitCode {
    if env::args_os().nth(1).is_some() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match run_local_update_fixture() {
        Ok(outcome) => {
            println!("{}", outcome.message());
            ExitCode::from(outcome.exit_code())
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
    fn command_surface_has_no_dynamic_update_input() {
        assert_eq!(USAGE, "usage: anodrel-local-update-fixture-acceptance");
        for absent in ["--application", "--endpoint", "--installer", "--path"] {
            assert!(!USAGE.contains(absent));
        }
    }
}
