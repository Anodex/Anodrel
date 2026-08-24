use super::support::*;
use crate::*;

#[test]
fn no_operation_or_capability_reaches_a_host_crash_record() {
    // Crash records are host-only by design: the diagnostic ledger is
    // readable behind `diagnostics.read`, and a crash record is readable
    // through nothing at all. Merging the two would put host defect
    // information behind a grant an application can hold. This is the
    // invariant most likely to be broken by someone adding a convenience,
    // so it is asserted rather than left to the absence of code.
    // See docs/CRASH_REPORTS.md and Decision 0065.
    for operation in [
        "crash.read",
        "crash.records.read",
        "crash.report",
        "diagnostics.crash.read",
        "host.crash.read",
    ] {
        let response = JsonValue::parse(
            &host_with_notifications(RecordingNotifications::default())
                .handle_json(&request_v1_13(operation, "{}")),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("operation.unsupported"),
            "{operation} was answered by this host"
        );
    }

    for capability in [
        Capability::DiagnosticsRead,
        Capability::UiDocumentWrite,
        Capability::UiEventsRead,
        Capability::SessionClose,
        Capability::ClipboardRead,
        Capability::ClipboardWrite,
        Capability::ExternalOpen,
        Capability::NetworkFetch,
        Capability::DialogOpenFile,
        Capability::DialogSaveFile,
        Capability::FileReadText,
        Capability::StorageStateRead,
        Capability::StorageStateReplace,
        Capability::StorageStateClear,
        Capability::CredentialRead,
        Capability::CredentialWrite,
        Capability::CredentialDelete,
        Capability::NotificationShow,
    ] {
        assert!(
            !capability.as_str().contains("crash"),
            "{} names a crash surface",
            capability.as_str()
        );
    }
}

#[test]
fn accepts_ping_and_formats_a_utc_timestamp() {
    let response = JsonValue::parse(&host(vec![]).handle_json(&request(
        "platform.ping",
        r#"{"sentAt":"2026-07-31T12:00:00.000Z"}"#,
    )))
    .expect("response JSON is valid");
    assert_eq!(field(&response, "status").as_string(), Some("success"));
    assert!(
        field(field(&response, "result"), "receivedAt")
            .as_string()
            .is_some_and(|timestamp| timestamp.ends_with('Z'))
    );
}

#[test]
fn rejects_forged_capability_context() {
    let response = JsonValue::parse(&host(vec![]).handle_json(&format!(
        r#"{},"capabilityContext":{{"grantedCapabilities":["diagnostics.read"]}}}}"#,
        request("platform.health", "{}")
            .strip_suffix('}')
            .expect("request ends with a brace")
    )))
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("capability.denied")
    );
}

#[test]
fn service_bundle_exposes_only_the_explicitly_attached_services() {
    let host = CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::CredentialWrite, Capability::StorageStateRead],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable()
            .with_credentials(MemoryCredentials::default())
            .with_storage(MemoryStorage::with_state(StorageRead::Absent)),
    );

    let credential = JsonValue::parse(&host.handle_json(&request_v1_12(
        "credential.write",
        r#"{"name":"refresh-token","secret":"00aaff"}"#,
    )))
    .expect("credential response is JSON");
    assert_eq!(
        field(field(&credential, "result"), "status").as_string(),
        Some("written")
    );

    let storage = JsonValue::parse(&host.handle_json(&request_v1_10("storage.state.read", "{}")))
        .expect("storage response is JSON");
    assert_eq!(
        field(field(&storage, "result"), "status").as_string(),
        Some("absent")
    );

    let unavailable = JsonValue::parse(&host.handle_json(&request_v1_5("clipboard.read", "{}")))
        .expect("clipboard response is JSON");
    assert_eq!(
        field(field(&unavailable, "error"), "code").as_string(),
        Some("capability.denied")
    );
}

#[test]
fn rejects_duplicate_host_capability_grants() {
    assert!(
        HostPolicy::new(
            "test.application",
            vec![Capability::DiagnosticsRead, Capability::DiagnosticsRead],
            "test-host",
        )
        .is_err()
    );
}

#[test]
fn rejects_unsupported_versions_and_oversized_messages() {
    let unsupported =
        request("platform.ping", r#"{"sentAt":"now"}"#).replacen("\"major\":1", "\"major\":2", 1);
    let response = JsonValue::parse(&host(vec![]).handle_json(&unsupported)).expect("valid JSON");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("protocol.version_unsupported")
    );

    let response = JsonValue::parse(&host(vec![]).handle_json(&"x".repeat(MAX_REQUEST_BYTES + 1)))
        .expect("valid JSON");
    assert_eq!(
        field(field(&response, "error"), "code").as_string(),
        Some("request.invalid")
    );
}

#[test]
fn converts_known_epoch_days_without_a_time_library() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_300), (2025, 7, 31));
}
