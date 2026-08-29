//! Rejection checks for grants unavailable to older record versions.

use super::*;

#[test]
fn an_earlier_record_cannot_name_network_fetch_or_network_origins() {
    let fixture = fixture();
    let record = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace("\"minor\": 0", "\"minor\": 13")
        .replace(
            "\"publisher\": {",
            "\"capabilities\": [\"network.fetch\"], \"publisher\": {",
        );
    fs::write(&fixture.record_path, record).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));

    let network_origins = fs::read_to_string(&fixture.record_path)
        .expect("record is read")
        .replace(
            "\"publisher\": {",
            "\"networkOrigins\": [], \"publisher\": {",
        )
        .replace("\"network.fetch\"", "\"diagnostics.read\"");
    fs::write(&fixture.record_path, network_origins).expect("record is updated");
    assert!(matches!(
        InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
        Err(InstalledApplicationError::InvalidRecord)
    ));
    fixture.remove();
}

#[test]
fn an_earlier_record_cannot_name_the_folder_entry_grant() {
    for minor in 0..=15 {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"folder.read_entries\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");
        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.16 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_state_read_grant() {
    for minor in 0..=16 {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.state.read\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");
        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.17 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_state_observe_grant() {
    for minor in 0..=17 {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.state.observe\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");
        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.18 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_session_window_grants() {
    for minor in 0..=12 {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.open\", \"window.close\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.13 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_size_grant() {
    // Version 1.11 is the newest record before this grant. Keeping it
    // invalid prevents a stale provisioning tool from silently widening a
    // verified application's session-window authority.
    for minor in ["8", "9", "10", "11"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.size\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.12 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_binary_file_write_grant() {
    for minor in ["7", "8", "9", "10"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"file.write_binary\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.11 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_fullscreen_grant() {
    // Version 1.9 is the newest record before this grant. Keeping it
    // invalid prevents a stale provisioning tool from silently widening a
    // verified application's desktop-presentation authority.
    for minor in ["6", "7", "8", "9"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.fullscreen\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.10 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_focus_grant() {
    // Version 1.8 is the newest record before this grant. Keeping it
    // invalid prevents a stale provisioning tool from silently widening a
    // verified application's attention authority.
    for minor in ["6", "7", "8"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.focus\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.9 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_menu_write_grant() {
    // Version 1.7 is the newest record before this grant. Keeping it
    // invalid prevents a provisioning tool from widening a verified
    // application before the new menu authority is intentionally granted.
    for minor in ["5", "6", "7"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"menu.write\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.8 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_file_text_write_grant() {
    // Version 1.6 is the newest record before this grant. Keeping it
    // invalid prevents a stale provisioning tool from silently widening a
    // verified application's file-system authority.
    for minor in ["4", "5", "6"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"file.write_text\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.7 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_field_read_grant() {
    // 1.4 is the case that matters: the newest version predating this
    // grant, so the one a stale provisioning step would still be writing.
    for minor in ["2", "3", "4"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"ui.fields.read\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.5 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_state_grant() {
    // Version 1.5 is the newest record before this grant. Keeping it
    // invalid prevents a stale provisioning tool from silently widening a
    // verified application's window authority.
    for minor in ["3", "4", "5"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.state\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.6 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_window_title_grant() {
    // The same widening guard as the notification grant, one version later.
    // Version 1.3 is the interesting case: it is the newest version that
    // predates this grant, so it is the one a stale provisioning step would
    // most plausibly still be writing.
    for minor in ["1", "2", "3"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"window.title\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(
            matches!(
                InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
                Err(InstalledApplicationError::InvalidRecord)
            ),
            "record version 1.{minor} accepted a 1.4 grant"
        );
        fixture.remove();
    }
}

#[test]
fn an_earlier_record_cannot_name_the_notification_grant() {
    // Provisioning must not be able to widen a record by naming a grant its
    // declared version does not carry.
    for minor in ["1", "2"] {
        let fixture = fixture();
        let record = fs::read_to_string(&fixture.record_path)
            .expect("record is read")
            .replace("\"minor\": 0", &format!("\"minor\": {minor}"))
            .replace(
                "\"publisher\": {",
                "\"capabilities\": [\"notification.show\"], \"publisher\": {",
            );
        fs::write(&fixture.record_path, record).expect("record is updated");

        assert!(matches!(
            InstalledApplication::load(&fixture.record_path, &fixture.policy_root),
            Err(InstalledApplicationError::InvalidRecord)
        ));
        fixture.remove();
    }
}
