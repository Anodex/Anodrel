//! Version 1.14 installed-record network-grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_14_couples_network_fetch_to_machine_selected_exact_origins() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 14")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.open\", \"window.close\", \"network.fetch\"], \"networkOrigins\": [{\"host\": \"Api.Example.Test\", \"port\": 443}, {\"host\": \"status.example.test\", \"port\": 8443}], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.14 network record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::WindowOpen,
            anodrel_protocol::Capability::WindowClose,
            anodrel_protocol::Capability::NetworkFetch,
        ]
    );
    let policy = installed
        .network_origin_policy()
        .expect("network grant carries an exact origin policy");
    assert_eq!(policy.origins().len(), 2);
    assert_eq!(policy.origins()[0].hostname(), "api.example.test");
    assert_eq!(policy.origins()[0].port(), 443);
    assert_eq!(policy.origins()[1].hostname(), "status.example.test");
    assert_eq!(policy.origins()[1].port(), 8443);
    fixture.remove();
}

#[test]
fn record_v1_14_requires_network_grant_and_origin_policy_to_agree() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 14")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [], \"networkOrigins\": [], \"publisher\": {",
        );
    fs::write(&fixture.record_path, &record).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("a network-free v1.14 record is valid");
    assert!(installed.network_origin_policy().is_none());

    let unused_origin = record.replace(
        "\"networkOrigins\": []",
        "\"networkOrigins\": [{\"host\": \"api.example.test\", \"port\": 443}]",
    );
    fs::write(&fixture.record_path, unused_origin).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));

    let missing_origin = record.replace(
        "\"capabilities\": []",
        "\"capabilities\": [\"network.fetch\"]",
    );
    fs::write(&fixture.record_path, missing_origin).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}

#[test]
fn record_v1_14_rejects_noncanonical_duplicate_and_malformed_network_origins() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 14")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"network.fetch\"], \"networkOrigins\": [{\"host\": \"api.example.test\", \"port\": 443}, {\"host\": \"API.EXAMPLE.TEST\", \"port\": 443}], \"publisher\": {",
        );
    fs::write(&fixture.record_path, &record).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));

    let malformed = record.replace(
        "\"API.EXAMPLE.TEST\", \"port\": 443",
        "\"api.example.test\", \"port\": 0",
    );
    fs::write(&fixture.record_path, malformed).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));

    let unexpected_field = record.replace(
        "\"host\": \"api.example.test\", \"port\": 443}, {\"host\": \"API.EXAMPLE.TEST\", \"port\": 443}",
        "\"host\": \"api.example.test\", \"port\": 443, \"path\": \"/status\"}",
    );
    fs::write(&fixture.record_path, unexpected_field).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
