//! Version 1.16 installed-record folder-entry grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_16_adds_only_the_separate_folder_entry_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 16")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"folder.read_entries\"], \"networkOrigins\": [], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.16 folder-entry record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::FolderReadEntries]
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 16", "\"minor\": 15");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
