//! A compiled development probe for the private Anodrel child transport.
//!
//! The host supplies this executable explicitly through its diagnostic command.
//! It reads exactly one bootstrap invitation, authenticates to its one invited
//! pipe, validates `platform.health`, and exits. It accepts no arguments,
//! configuration, files, network input, or application content. It is not an
//! application runtime, package format, or product executable.

#![deny(unsafe_op_in_unsafe_fn)]

mod stages;

use std::{io, process::ExitCode};

use anodrel_client::{Client, ProtocolVersion};
use anodrel_json::JsonValue;
use anodrel_windows_client::WindowsClientStream;

use stages::Stage;

type NativeClient = Client<WindowsClientStream>;

fn main() -> ExitCode {
    ExitCode::from(run().code())
}

fn run() -> Stage {
    let Ok(invitation) = NativeClient::read_invitation(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Ok(stream) = WindowsClientStream::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    let Ok(mut client) = NativeClient::authenticate(stream, invitation) else {
        return Stage::AuthenticationRejected;
    };
    let Ok(health) = client.request(
        ProtocolVersion::v1(0),
        "native-sample-health",
        "platform.health",
        JsonValue::Object(Default::default()),
    ) else {
        return Stage::HealthRejected;
    };
    if validates_health(&health) {
        Stage::Completed
    } else {
        Stage::HealthRejected
    }
}

/// Verifies the closed facts promised by `platform.health` without copying any
/// host response into output. A non-empty host name is enough here: the host
/// chooses it, and this probe must remain reusable by the integration test.
fn validates_health(value: &JsonValue) -> bool {
    let Some(fields) = value.as_object() else {
        return false;
    };
    let Some(version) = fields.get("protocolVersion").and_then(JsonValue::as_object) else {
        return false;
    };
    fields.get("status").and_then(JsonValue::as_string) == Some("ready")
        && fields
            .get("hostName")
            .and_then(JsonValue::as_string)
            .is_some_and(|name| !name.is_empty())
        // A request at Protocol 1.0 can receive a response from a newer
        // compatible Protocol 1 host. The major is the compatibility boundary;
        // the reported minor is informational and must not pin this probe to
        // the version that first introduced `platform.health`.
        && version.get("major").and_then(JsonValue::as_u16) == Some(1)
        && version.get("minor").and_then(JsonValue::as_u16).is_some()
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;

    use super::validates_health;

    #[test]
    fn accepts_only_a_complete_ready_health_result() {
        let ready = JsonValue::parse(
            r#"{"status":"ready","hostName":"test-host","protocolVersion":{"major":1,"minor":0}}"#,
        )
        .expect("fixture is JSON");
        assert!(validates_health(&ready));

        let compatible_newer_minor = JsonValue::parse(
            r#"{"status":"ready","hostName":"test-host","protocolVersion":{"major":1,"minor":19}}"#,
        )
        .expect("fixture is JSON");
        assert!(validates_health(&compatible_newer_minor));

        let wrong_major = JsonValue::parse(
            r#"{"status":"ready","hostName":"test-host","protocolVersion":{"major":2,"minor":0}}"#,
        )
        .expect("fixture is JSON");
        assert!(!validates_health(&wrong_major));

        let missing_name =
            JsonValue::parse(r#"{"status":"ready","protocolVersion":{"major":1,"minor":0}}"#)
                .expect("fixture is JSON");
        assert!(!validates_health(&missing_name));
        assert!(!validates_health(&JsonValue::Null));
    }
}
