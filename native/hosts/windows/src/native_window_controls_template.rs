//! Fixed route for one generated native window-controls development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_window_controls(
    "anodrel.native-window-controls-template",
    "native-window-controls-template-session",
    "Anodrel Native Window Controls Template",
    "Anodrel native window-controls template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with only the five existing targetless session-window controls,
/// document replacement, semantic-action reading, and self-close.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
