//! Version 1.18 installed-record state-change grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_18_adds_only_the_separate_window_state_observe_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 18")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.state.observe\"], \"networkOrigins\": [], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.18 state-change record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::WindowStateObserve]
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 18", "\"minor\": 17");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
