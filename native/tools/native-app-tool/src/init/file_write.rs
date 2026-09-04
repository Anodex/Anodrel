//! File-write project initialization entry point.

use std::path::Path;

use super::{InitError, initialize_template};
use crate::arguments::TemplateKind;

/// Creates one constrained native project for retained selected-output writing.
pub fn initialize_file_write(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(
        TemplateKind::FileWrite,
        destination,
        project_slug,
        display_label,
    )
}
