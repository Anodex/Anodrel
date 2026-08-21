//! Fixed route for one generated native UI development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::new(
    "anodrel.native-template",
    "native-template-session",
    "Anodrel Native Template",
    "Anodrel native template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with only document replacement, semantic-action reading, and self-close.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
