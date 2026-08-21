#![forbid(unsafe_code)]

//! First-party authoring and verification for the current Anodrel text package.

mod arguments;
mod commands;

use std::env;

use arguments::parse;
use commands::run;

const USAGE: &str = concat!(
    "usage:\n",
    "  anodrel-package-tool init <destination> <application-id> <display-name> [content]\n",
    "  anodrel-package-tool verify <anodrel.application.json>"
);

fn main() {
    let result = match parse(env::args().skip(1)) {
        Ok(command) => run(command).map_err(|error| error.to_string()),
        Err(_) => Err(USAGE.to_owned()),
    };
    if let Err(error) = result {
        eprintln!("anodrel-package-tool: {error}");
        std::process::exit(2);
    }
}
