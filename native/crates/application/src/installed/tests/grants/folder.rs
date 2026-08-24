//! Version 1.15 installed-record folder-dialog grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_15_adds_folder_selection_and_keeps_network_policy_explicit() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 15")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"dialog.open_folder\", \"network.fetch\"], \"networkOrigins\": [{\"host\": \"api.example.test\", \"port\": 443}], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.15 folder record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::DialogOpenFolder,
            anodrel_protocol::Capability::NetworkFetch,
        ]
    );
    assert_eq!(
        installed
            .network_origin_policy()
            .expect("network grant retains its exact origin policy")
            .origins()[0]
            .hostname(),
        "api.example.test"
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 15", "\"minor\": 14");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
