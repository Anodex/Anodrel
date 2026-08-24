use super::support::*;
use crate::*;

#[test]
fn storage_operations_are_exact_bounded_and_independently_granted() {
    let storage_host = storage_host(
        vec![
            Capability::StorageStateRead,
            Capability::StorageStateReplace,
            Capability::StorageStateClear,
        ],
        MemoryStorage::with_state(StorageRead::Absent),
    );
    let replaced = JsonValue::parse(&storage_host.handle_json(&request_v1_10(
        "storage.state.replace",
        r#"{"snapshot":"saved"}"#,
    )))
    .expect("replace response is JSON");
    assert_eq!(
        field(field(&replaced, "result"), "status").as_string(),
        Some("replaced")
    );

    let read =
        JsonValue::parse(&storage_host.handle_json(&request_v1_10("storage.state.read", "{}")))
            .expect("read response is JSON");
    assert_eq!(
        field(field(&read, "result"), "snapshot").as_string(),
        Some("saved")
    );

    let cleared =
        JsonValue::parse(&storage_host.handle_json(&request_v1_10("storage.state.clear", "{}")))
            .expect("clear response is JSON");
    assert_eq!(
        field(field(&cleared, "result"), "status").as_string(),
        Some("cleared")
    );

    let invalid = JsonValue::parse(
        &storage_host.handle_json(&request_v1_10("storage.state.read", r#"{"extra":true}"#)),
    )
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let no_grant =
        JsonValue::parse(&host(vec![]).handle_json(&request_v1_10("storage.state.read", "{}")))
            .expect("denied response is JSON");
    assert_eq!(
        field(field(&no_grant, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let oversized = format!(
        r#"{{"snapshot":"{}"}}"#,
        "x".repeat(MAX_STORAGE_SNAPSHOT_REQUEST_BYTES + 1)
    );
    let rejected = JsonValue::parse(
        &storage_host.handle_json(&request_v1_10("storage.state.replace", &oversized)),
    )
    .expect("oversized response is JSON");
    assert_eq!(
        field(field(&rejected, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );
}

#[test]
fn credential_operations_are_exact_and_independently_granted() {
    let service_host = credential_host(
        vec![
            Capability::CredentialRead,
            Capability::CredentialWrite,
            Capability::CredentialDelete,
        ],
        MemoryCredentials::default(),
    );
    let absent = JsonValue::parse(&service_host.handle_json(&request_v1_12(
        "credential.read",
        r#"{"name":"refresh-token"}"#,
    )))
    .expect("absent response is JSON");
    assert_eq!(
        field(field(&absent, "result"), "status").as_string(),
        Some("not_found")
    );

    let written = JsonValue::parse(&service_host.handle_json(&request_v1_12(
        "credential.write",
        r#"{"name":"refresh-token","secret":"00aaff"}"#,
    )))
    .expect("write response is JSON");
    assert_eq!(
        field(field(&written, "result"), "status").as_string(),
        Some("written")
    );

    let found = JsonValue::parse(&service_host.handle_json(&request_v1_12(
        "credential.read",
        r#"{"name":"refresh-token"}"#,
    )))
    .expect("read response is JSON");
    assert_eq!(
        field(field(&found, "result"), "secret").as_string(),
        Some("00aaff")
    );

    let deleted = JsonValue::parse(&service_host.handle_json(&request_v1_12(
        "credential.delete",
        r#"{"name":"refresh-token"}"#,
    )))
    .expect("delete response is JSON");
    assert_eq!(
        field(field(&deleted, "result"), "status").as_string(),
        Some("deleted")
    );

    let invalid = JsonValue::parse(&service_host.handle_json(&request_v1_12(
        "credential.write",
        r#"{"name":"refresh-token","secret":"ABCDEF"}"#,
    )))
    .expect("invalid response is JSON");
    assert_eq!(
        field(field(&invalid, "error"), "code").as_string(),
        Some("request.payload_invalid")
    );

    let denied = JsonValue::parse(
        &credential_host(
            vec![Capability::CredentialRead],
            MemoryCredentials::default(),
        )
        .handle_json(&request_v1_12(
            "credential.write",
            r#"{"name":"refresh-token","secret":"00aaff"}"#,
        )),
    )
    .expect("denied response is JSON");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    let unsupported = JsonValue::parse(&service_host.handle_json(&request_v1_10(
        "credential.read",
        r#"{"name":"refresh-token"}"#,
    )))
    .expect("unsupported response is JSON");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}
