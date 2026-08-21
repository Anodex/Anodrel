//! Fixed route for the compiled native UI-session diagnostic.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::new(
    "anodrel.native-ui-client-sample",
    "native-ui-client-sample-session",
    "Anodrel Native UI Probe",
    "Anodrel native UI development probe completed successfully.",
);

/// Runs the selected compiled diagnostic through one real native UI session.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
