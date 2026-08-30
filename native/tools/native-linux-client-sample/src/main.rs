//! A fixed development probe for one private Linux child transport session.
//!
//! It reads one ANLI record from standard input, opens only the invited socket,
//! authenticates, requests platform.health, and exits. It accepts no command
//! arguments or configuration and writes no sensitive output.

#[cfg(target_os = "linux")]
mod stages;

#[cfg(target_os = "linux")]
use std::{io, process::ExitCode};

#[cfg(target_os = "linux")]
use anodrel_client::{Client, ProtocolVersion};
#[cfg(target_os = "linux")]
use anodrel_json::JsonValue;
#[cfg(target_os = "linux")]
use anodrel_linux_client::{LinuxBootstrapInvitation, LinuxClientStream};
#[cfg(target_os = "linux")]
use stages::Stage;

#[cfg(target_os = "linux")]
type LinuxClient = Client<LinuxClientStream>;

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    ExitCode::from(run().code())
}

#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
fn run() -> Stage {
    let Ok(invitation) = LinuxBootstrapInvitation::read_from(&mut io::stdin()) else {
        return Stage::BootstrapUnreadable;
    };
    let Ok(stream) = LinuxClientStream::connect(&invitation) else {
        return Stage::EndpointUnavailable;
    };
    let Ok(mut client) = LinuxClient::authenticate(stream, invitation) else {
        return Stage::AuthenticationRejected;
    };
    let Ok(health) = client.request(
        ProtocolVersion::v1(0),
        "native-linux-sample-health",
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

#[cfg(target_os = "linux")]
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
        && version.get("major").and_then(JsonValue::as_u16) == Some(1)
        && version.get("minor").and_then(JsonValue::as_u16).is_some()
}

#[cfg(all(test, target_os = "linux"))]
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

        let wrong_major = JsonValue::parse(
            r#"{"status":"ready","hostName":"test-host","protocolVersion":{"major":2,"minor":0}}"#,
        )
        .expect("fixture is JSON");
        assert!(!validates_health(&wrong_major));
        assert!(!validates_health(&JsonValue::Null));
    }
}
