//! Version 1.19 installed-record context-menu grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_19_adds_only_the_separate_context_menu_write_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 19")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"menu.context.write\"], \"networkOrigins\": [], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.19 context-menu record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::ContextMenuWrite]
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 19", "\"minor\": 18");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
