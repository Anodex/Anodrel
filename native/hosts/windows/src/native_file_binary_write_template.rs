//! Fixed route for one generated native retained binary-output template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig =
    DevelopmentUiSessionConfig::with_file_binary_write(
        "anodrel.native-file-binary-write-template",
        "native-file-binary-write-template-session",
        "Anodrel Native Binary File Write Template",
        "Anodrel native binary file-write template session completed successfully.",
    );

/// Runs one selected executable with one retained binary-output write.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
