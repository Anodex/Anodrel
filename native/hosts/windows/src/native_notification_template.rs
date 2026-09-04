//! Fixed route for one generated native notification development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_notification(
    "anodrel.native-notification-template",
    "native-notification-template-session",
    "Anodrel Native Notification Template",
    "Anodrel native notification template session completed successfully.",
);

/// Runs one selected generated executable with document replacement, one-way
/// notification delivery, and self-close only.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
