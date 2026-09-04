//! Binary file-write project initialization entry point.

use std::path::Path;

use super::{InitError, initialize_template};
use crate::arguments::TemplateKind;

/// Creates one constrained native project for retained binary output writing.
pub fn initialize_file_binary_write(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(
        TemplateKind::FileBinaryWrite,
        destination,
        project_slug,
        display_label,
    )
}
