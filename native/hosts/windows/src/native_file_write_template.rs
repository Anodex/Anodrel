//! Fixed route for one generated native retained-output development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_file_write(
    "anodrel.native-file-write-template",
    "native-file-write-template-session",
    "Anodrel Native File Write Template",
    "Anodrel native file-write template session completed successfully.",
);

/// Runs one selected generated executable with one retained text-output write.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
