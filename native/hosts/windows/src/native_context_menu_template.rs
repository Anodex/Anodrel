//! Fixed route for one generated native context-menu development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_context_menu(
    "anodrel.native-context-menu-template",
    "native-context-menu-template-session",
    "Anodrel Native Context Menu Template",
    "Anodrel native context-menu template session completed successfully.",
);

/// Runs one selected generated executable as unverified development code with
/// document replacement, semantic context-menu replacement and action reads,
/// and self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
