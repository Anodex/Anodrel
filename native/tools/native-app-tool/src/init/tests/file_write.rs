//! File-write project scaffolding verification.

use std::fs;

use super::super::initialize_file_write;
use super::TestDirectory;

#[test]
fn file_write_project_has_no_general_filesystem_or_event_surface() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-file-write-app");
    initialize_file_write(
        &destination,
        "generated-file-write-app",
        "Generated File Write App",
    )
    .expect("generate a native file-write project");

    let source = fs::read_to_string(destination.join("src/main.rs"))
        .expect("generated file-write source is readable");
    let readme = fs::read_to_string(destination.join("README.md"))
        .expect("generated file-write readme is readable");
    assert!(source.contains("select_save_file_v2"));
    assert!(source.contains("write_selected_text"));
    assert!(!source.contains("read_actions"));
    assert!(!source.contains("std::fs"));
    assert!(readme.contains("retained"));
    assert!(!readme.contains("init-notification"));
}
