//! Fixed route for one generated native scroll-window development template.

use std::error::Error;

use crate::development_ui_session::{DevelopmentUiSessionConfig, run as run_development_session};

const CONFIGURATION: DevelopmentUiSessionConfig = DevelopmentUiSessionConfig::with_multi_window(
    "anodrel.native-scroll-window-template",
    "native-scroll-window-template-session",
    "Anodrel Native Scroll Window Template",
    "Anodrel native scroll-window template session completed successfully.",
);

/// Runs one explicitly selected generated executable as unverified development
/// code with strict-v2 secondary documents and the fixed multi-window grants.
pub fn run(client_path: &str) -> Result<(), Box<dyn Error>> {
    run_development_session(client_path, CONFIGURATION)
}
