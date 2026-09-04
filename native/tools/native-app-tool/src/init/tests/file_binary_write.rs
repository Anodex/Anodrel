//! Binary file-write project scaffolding verification.

use std::fs;

use super::super::initialize_file_binary_write;
use super::TestDirectory;

#[test]
fn binary_file_write_project_has_no_text_or_general_filesystem_surface() {
    let temporary = TestDirectory::new();
    let destination = temporary.path.join("generated-file-binary-write-app");
    initialize_file_binary_write(
        &destination,
        "generated-file-binary-write-app",
        "Generated File Binary Write App",
    )
    .expect("generate a native binary file-write project");

    let source = fs::read_to_string(destination.join("src/main.rs"))
        .expect("generated binary file-write source is readable");
    let readme = fs::read_to_string(destination.join("README.md"))
        .expect("generated binary file-write readme is readable");
    assert!(source.contains("write_selected_binary"));
    assert!(source.contains("FileBinaryData::decode_base64url"));
    assert!(!source.contains("write_selected_text"));
    assert!(!source.contains("std::fs"));
    assert!(readme.contains("retained"));
    assert!(!readme.contains("init-file-write"));
}
