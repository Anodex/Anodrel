//! Fixed route for one generated native multi-window development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_multi_window(
    "anodrel.native-multi-window-template",
    "native-multi-window-template-session",
    "Anodrel Native Multi-Window Template",
    "Anodrel native multi-window template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with document replacement, tagged action reads, bounded secondary
/// creation and close, and self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
