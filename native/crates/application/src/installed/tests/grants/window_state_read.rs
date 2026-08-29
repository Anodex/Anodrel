//! Version 1.17 installed-record state-observation grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_17_adds_only_the_separate_window_state_read_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 17")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.state.read\"], \"networkOrigins\": [], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.17 state-observation record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::WindowStateRead]
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 17", "\"minor\": 16");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
