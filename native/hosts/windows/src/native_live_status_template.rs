//! Fixed route for one generated native live-status development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::new(
    "anodrel.native-live-status-template",
    "native-live-status-template-session",
    "Anodrel Native Live Status Template",
    "Anodrel native live-status template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with version-3 document replacement, semantic-action reading, and
/// self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
