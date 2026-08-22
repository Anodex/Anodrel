//! Fixed route for one generated native form development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_form(
    "anodrel.native-form-template",
    "native-form-template-session",
    "Anodrel Native Form Template",
    "Anodrel native form template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with document replacement, semantic-action read, whole-surface field
/// read, and self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
