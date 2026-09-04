//! Fixed route for one generated native tray development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_tray(
    "anodrel.native-tray-template",
    "native-tray-template-session",
    "Anodrel Native Tray Template",
    "Anodrel native tray template session completed successfully.",
);

/// Runs one selected generated executable with document replacement, semantic
/// tray replacement and action reads, and self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
