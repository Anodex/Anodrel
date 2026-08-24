use super::support::*;
use crate::*;

#[test]
fn a_granted_complete_menu_reaches_only_the_menu_service() {
    let service = RecordingMenu::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_menu(service);
    let first_payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
    let first = JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", first_payload)))
        .expect("response JSON is valid");
    assert_eq!(field(&first, "status").as_string(), Some("success"));
    assert_eq!(
        field(field(&first, "result"), "revision").as_string(),
        Some("1")
    );

    let second = JsonValue::parse(&host.handle_json(&request_v1_18(
        "menu.replace",
        r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":false}]}]}"#,
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&second, "result"), "revision").as_string(),
        Some("2")
    );

    let replacements = replacements
        .lock()
        .expect("menu recorder lock is available");
    assert_eq!(replacements.len(), 2);
    assert_eq!(replacements[0].0.value(), 1);
    assert_eq!(replacements[0].1.menus()[0].label().as_str(), "File");
    assert!(replacements[0].1.menus()[0].items()[0].enabled());
    assert_eq!(replacements[1].0.value(), 2);
    assert!(!replacements[1].1.menus()[0].items()[0].enabled());
}

#[test]
fn a_menu_needs_its_own_grant_protocol_version_and_host_surface() {
    let payload = r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#;
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::WindowState],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_menu(RecordingMenu::default()),
        )
        .handle_json(&request_v1_18("menu.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(
        &host_with_menu(RecordingMenu::default())
            .handle_json(&request_v1_17("menu.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );

    let unavailable = JsonValue::parse(
        &host_with_menu(RecordingMenu::unavailable())
            .handle_json(&request_v1_18("menu.replace", payload)),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("menu.unavailable")
    );
}

#[test]
fn a_v1_24_menu_shortcut_is_canonical_unique_and_version_gated() {
    let service = RecordingMenu::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_menu(service);
    let payload = r#"{"menus":[{"label":"File","items":[{"id":"document.complete","label":"Complete","enabled":true,"shortcut":"Ctrl+Shift+M"}]}]}"#;
    let accepted = JsonValue::parse(&host.handle_json(&request_v1_24("menu.replace", payload)))
        .expect("response JSON is valid");
    assert_eq!(field(&accepted, "status").as_string(), Some("success"));
    let replacements = replacements
        .lock()
        .expect("menu recorder lock is available");
    assert_eq!(
        replacements[0].1.menus()[0].items()[0]
            .shortcut()
            .expect("shortcut is retained")
            .display_text(),
        "Ctrl+Shift+M"
    );
    drop(replacements);

    let old_version = JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", payload)))
        .expect("response JSON is valid");
    assert_eq!(
        field(field(&old_version, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    for invalid in [
        r#"{"menus":[{"label":"File","items":[{"id":"document.complete","label":"Complete","enabled":true,"shortcut":"Ctrl+m"}]}]}"#,
        r#"{"menus":[{"label":"File","items":[{"id":"document.primary","label":"Primary","enabled":true,"shortcut":"Ctrl+M"},{"id":"document.secondary","label":"Secondary","enabled":false,"shortcut":"Ctrl+M"}]}]}"#,
    ] {
        let response = JsonValue::parse(&host.handle_json(&request_v1_24("menu.replace", invalid)))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{invalid} was accepted"
        );
    }
}

#[test]
fn a_menu_payload_is_exact_bounded_and_cannot_name_native_behavior() {
    let service = RecordingMenu::default();
    let replacements = Arc::clone(&service.replacements);
    let host = host_with_menu(service);
    for payload in [
        r#"{}"#,
        r#"{"menus":[]}"#,
        r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":"true"}]}]}"#,
        r#"{"menus":[{"label":"File","items":[{"id":"document.new","label":"New document","enabled":true,"nativeId":1}]}]}"#,
        r#"{"menus":[{"label":"File\nOpen","items":[{"id":"document.new","label":"New document","enabled":true}]}]}"#,
        r#"{"menus":[{"label":"File","items":[{"id":"native command","label":"New document","enabled":true}]}]}"#,
    ] {
        let response = JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", payload)))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }

    let items = (0..16)
        .map(|item| {
            format!(
                r#"{{"id":"command{item}","label":"{}","enabled":true}}"#,
                "x".repeat(96)
            )
        })
        .collect::<Vec<_>>();
    let menus = (0..8)
        .map(|menu| format!(r#"{{"label":"Menu{menu}","items":[{}]}}"#, items.join(",")))
        .collect::<Vec<_>>();
    let oversized = format!(r#"{{"menus":[{}]}}"#, menus.join(","));
    assert!(oversized.len() > MAX_MENU_REPLACE_REQUEST_BYTES);
    let response = JsonValue::parse(&host.handle_json(&request_v1_18("menu.replace", &oversized)))
        .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
    assert!(
        replacements
            .lock()
            .expect("menu recorder lock is available")
            .is_empty()
    );
}
