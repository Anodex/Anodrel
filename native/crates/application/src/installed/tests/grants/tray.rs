//! Version 1.24 installed-record tray grant compatibility checks.

use super::super::*;

#[test]
fn record_v1_24_adds_only_the_separate_tray_write_grant() {
    let fixture = fixture();
    let launcher_path = fixture.package_root.join("bin").join("launcher.exe");
    let launcher = b"Anodrel fixture launcher";
    fs::write(&launcher_path, launcher).expect("fixture launcher is written");
    let launcher_digest = crate::sha256::to_lower_hex(&crate::sha256::digest(launcher));
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 24")
        .replace(
            "\"publisher\": {",
            &format!(
                "\"capabilities\": [\"tray.write\"], \"networkOrigins\": [], \"updateCatalogue\": {{\"origin\": {{\"host\": \"updates.example.test\", \"port\": 443}}, \"path\": \"/catalogue.p7s\"}}, \"product\": {{\"displayName\": \"Test Application\", \"publisherName\": \"Test Publisher\", \"startMenuName\": \"Test Application\"}}, \"launcher\": {{\"path\": \"bin/launcher.exe\", \"sha256\": \"{launcher_digest}\"}}, \"publisher\": {{",
            ),
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.24 tray record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::TrayWrite]
    );

    let older = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 24", "\"minor\": 23");
    fs::write(&fixture.record_path, older).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}
