//! Fixed route for one generated native menu development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_menu(
    "anodrel.native-menu-template",
    "native-menu-template-session",
    "Anodrel Native Menu Template",
    "Anodrel native menu template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with document replacement, event reading, menu replacement, and
/// self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
