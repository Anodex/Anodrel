//! Versioned installed-record grant compatibility checks.

use super::*;

mod folder;
mod folder_access;
mod network;

#[test]
fn loads_a_record_that_binds_package_executable_and_publisher() {
    let fixture = fixture();
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("installed record is valid");

    assert_eq!(installed.identity().application_id(), APPLICATION_ID);
    assert_eq!(
        installed.executable_path(),
        fs::canonicalize(&fixture.executable_path)
            .as_deref()
            .expect("executable canonicalizes")
    );
    assert!(installed.matches_executable_digest(sha256::digest(b"Anodrel fixture executable")));
    assert!(installed.matches_publisher([0xA5; 32]));
    assert!(!installed.matches_publisher([0x5A; 32]));
    assert_eq!(format!("{installed:?}"), "InstalledApplication(..)");
    fixture.remove();
}

#[test]
fn loads_a_trusted_operating_system_record_with_a_matching_identity() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path).expect("record is read");
    let installed = InstalledApplication::load_from_trusted_record(&record, APPLICATION_ID)
        .expect("trusted record is valid");

    assert_eq!(installed.identity().application_id(), APPLICATION_ID);
    fixture.remove();
}

#[test]
fn revalidation_hashes_the_record_executable_and_rejects_a_substitute_path() {
    let fixture = fixture();
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("installed record is valid");
    let mut executable = fs::File::open(&fixture.executable_path).expect("executable opens");
    installed
        .revalidate_executable(&fixture.executable_path, &mut executable)
        .expect("record executable revalidates");

    let substitute = fixture.package_root.join("bin").join("substitute.exe");
    fs::write(&substitute, b"Anodrel fixture executable").expect("substitute is written");
    let mut substitute_file = fs::File::open(&substitute).expect("substitute opens");
    assert!(matches!(
        installed.revalidate_executable(&substitute, &mut substitute_file),
        Err(InstalledApplicationError::ExecutablePathChanged)
    ));
    fixture.remove();
}

#[test]
fn record_v1_1_accepts_only_supported_machine_grants() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path).expect("record is read");
    let record = record.replace("\"minor\": 0", "\"minor\": 1").replace(
        "\"publisher\": {",
        "\"capabilities\": [\"diagnostics.read\"], \"publisher\": {",
    );
    fs::write(&fixture.record_path, record).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.1 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::DiagnosticsRead]
    );

    let ui_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("diagnostics.read", "ui.document.write");
    fs::write(&fixture.record_path, ui_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("UI document grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::UiDocumentWrite]
    );

    let event_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("ui.document.write", "ui.events.read");
    fs::write(&fixture.record_path, event_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("UI event grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::UiEventsRead]
    );

    let close_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("ui.events.read", "session.close");
    fs::write(&fixture.record_path, close_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("session close grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::SessionClose]
    );

    let clipboard_read_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("session.close", "clipboard.read");
    fs::write(&fixture.record_path, clipboard_read_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("clipboard read grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::ClipboardRead]
    );

    let clipboard_write_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("clipboard.read", "clipboard.write");
    fs::write(&fixture.record_path, clipboard_write_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("clipboard write grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::ClipboardWrite]
    );

    let external_open_grant = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("clipboard.write", "external.open");
    fs::write(&fixture.record_path, external_open_grant).expect("record is updated");
    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("external open grant is valid");
    assert_eq!(
        installed.capabilities(),
        &[anodrel_protocol::Capability::ExternalOpen]
    );

    let unsupported = fs::read_to_string(&fixture.record_path)
        .expect("validated record is read")
        .replace("external.open", "credential.read");
    fs::write(&fixture.record_path, unsupported).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}

#[test]
fn record_v1_2_accepts_the_newly_composable_storage_and_credential_grants() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 2")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"storage.state.read\", \"storage.state.replace\", \"storage.state.clear\", \"credential.read\", \"credential.write\", \"credential.delete\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.2 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::StorageStateRead,
            anodrel_protocol::Capability::StorageStateReplace,
            anodrel_protocol::Capability::StorageStateClear,
            anodrel_protocol::Capability::CredentialRead,
            anodrel_protocol::Capability::CredentialWrite,
            anodrel_protocol::Capability::CredentialDelete,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_3_adds_notifications_and_keeps_every_earlier_grant() {
    // Each version is a superset, so a record written for 1.2 must keep its
    // exact meaning when its version is raised.
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 3")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"credential.read\", \"notification.show\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.3 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::CredentialRead,
            anodrel_protocol::Capability::NotificationShow,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_4_adds_the_window_title_grant_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 4")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"credential.read\", \"notification.show\", \"window.title\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.4 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::CredentialRead,
            anodrel_protocol::Capability::NotificationShow,
            anodrel_protocol::Capability::WindowTitle,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_5_adds_the_field_read_grant_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 5")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"notification.show\", \"window.title\", \"ui.fields.read\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.5 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::NotificationShow,
            anodrel_protocol::Capability::WindowTitle,
            anodrel_protocol::Capability::UiFieldsRead,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_6_adds_window_state_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 6")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"window.title\", \"ui.fields.read\", \"window.state\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.6 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::WindowTitle,
            anodrel_protocol::Capability::UiFieldsRead,
            anodrel_protocol::Capability::WindowState,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_7_adds_file_text_write_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 7")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"window.state\", \"file.write_text\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.7 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::WindowState,
            anodrel_protocol::Capability::FileWriteText,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_8_adds_menu_write_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 8")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"clipboard.read\", \"file.write_text\", \"menu.write\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.8 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::ClipboardRead,
            anodrel_protocol::Capability::FileWriteText,
            anodrel_protocol::Capability::MenuWrite,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_9_adds_window_focus_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 9")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.state\", \"menu.write\", \"window.focus\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.9 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::WindowState,
            anodrel_protocol::Capability::MenuWrite,
            anodrel_protocol::Capability::WindowFocus,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_10_adds_window_fullscreen_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 10")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.state\", \"window.focus\", \"window.fullscreen\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.10 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::WindowState,
            anodrel_protocol::Capability::WindowFocus,
            anodrel_protocol::Capability::WindowFullscreen,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_11_adds_binary_file_write_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 11")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"file.write_text\", \"window.fullscreen\", \"file.write_binary\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.11 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::FileWriteText,
            anodrel_protocol::Capability::WindowFullscreen,
            anodrel_protocol::Capability::FileWriteBinary,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_12_adds_window_size_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 12")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"file.write_binary\", \"window.fullscreen\", \"window.size\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.12 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::FileWriteBinary,
            anodrel_protocol::Capability::WindowFullscreen,
            anodrel_protocol::Capability::WindowSize,
        ]
    );
    fixture.remove();
}

#[test]
fn record_v1_13_adds_session_window_grants_and_keeps_every_earlier_grant() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 13")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"window.size\", \"window.open\", \"window.close\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");

    let installed = InstalledApplication::load(&fixture.record_path, &fixture.policy_root)
        .expect("v1.13 record is valid");
    assert_eq!(
        installed.capabilities(),
        &[
            anodrel_protocol::Capability::WindowSize,
            anodrel_protocol::Capability::WindowOpen,
            anodrel_protocol::Capability::WindowClose,
        ]
    );
    fixture.remove();
}
