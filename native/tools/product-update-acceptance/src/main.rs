//! Command-line entry point for the fixed development update acceptance runner.

use std::{env, process::ExitCode};

use anodrel_product_update_acceptance::run;

const USAGE: &str = "usage: anodrel-product-update-acceptance";

fn main() -> ExitCode {
    if env::args().nth(1).is_some() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match run() {
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
    fn usage_accepts_no_selector_or_update_input() {
        assert_eq!(USAGE, "usage: anodrel-product-update-acceptance");
        for absent in ["--application", "--origin", "--path", "--installer"] {
            assert!(!USAGE.contains(absent));
        }
    }
}
