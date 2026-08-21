#![forbid(unsafe_code)]

//! First-party scaffolding for the constrained Anodrel native UI template.
//!
//! The tool writes a new Rust project only. It neither runs the project nor
//! changes machine policy, signing, installation, or host capabilities.

mod arguments;
mod init;
mod paths;
mod template;
mod validation;

use std::env;

use arguments::parse;
use init::initialize;

const USAGE: &str = concat!(
    "usage:\n",
    "  anodrel-native-app-tool init <destination> <project-slug> <display-label>"
);

fn main() {
    let result = match parse(env::args().skip(1)) {
        Ok(command) => initialize(
            &command.destination,
            &command.project_slug,
            &command.display_label,
        )
        .map_err(|error| error.to_string()),
        Err(()) => Err(USAGE.to_owned()),
    };
    if let Err(error) = result {
        eprintln!("anodrel-native-app-tool: {error}");
        std::process::exit(2);
    }
}
