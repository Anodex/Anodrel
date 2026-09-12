#![deny(missing_docs)]

//! Command-line entry point for the fixed local signed-update fixture server.

use std::{env, process::ExitCode};

use anodrel_local_update_fixture_server::LocalUpdateFixtureServer;

const USAGE: &str = "usage: anodrel-local-update-fixture-server";

fn main() -> ExitCode {
    if env::args_os().nth(1).is_some() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let outcome = LocalUpdateFixtureServer::bind().and_then(|server| {
        println!("Anodrel local update fixture server is listening. Press Ctrl+C to stop it.");
        server.serve()
    });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
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
    fn command_surface_has_no_listener_configuration() {
        assert_eq!(USAGE, "usage: anodrel-local-update-fixture-server");
        for absent in ["--port", "--root", "--path", "--certificate"] {
            assert!(!USAGE.contains(absent));
        }
    }
}
