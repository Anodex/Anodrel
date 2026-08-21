//! A compiled development diagnostic for one fixed HTTPS text fetch.
//!
//! The host supplies this executable explicitly through its diagnostic command.
//! It reads exactly one bootstrap invitation, authenticates to its one invited
//! pipe, requests only the compiled public URL, validates the bounded protocol
//! result, and exits. It accepts no arguments, configuration, files, or network
//! input. It is not a general HTTP client, application runtime, or product
//! executable.

#![deny(unsafe_op_in_unsafe_fn)]

mod stages;

use std::{collections::BTreeMap, io, process::ExitCode};

use anodrel_client::{Client, ProtocolVersion};
use anodrel_json::JsonValue;
use anodrel_network::MAX_NETWORK_TEXT_BYTES;
use anodrel_windows_client::WindowsClientStream;

use stages::Stage;

const DIAGNOSTIC_URL: &str = "https://example.com/";
const REQUEST_ID: &str = "native-network-sample-fetch";

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
    let Ok(response) = client.request(
        ProtocolVersion::v1(19),
        REQUEST_ID,
        "network.fetch_text",
        fetch_payload(),
    ) else {
        return Stage::FetchRejected;
    };
    if validates_fetch_response(&response) {
        Stage::Completed
    } else {
        Stage::FetchRejected
    }
}

fn fetch_payload() -> JsonValue {
    JsonValue::Object(BTreeMap::from([(
        "url".to_owned(),
        JsonValue::String(DIAGNOSTIC_URL.to_owned()),
    )]))
}

/// Verifies exactly the public result shape without retaining or printing the
/// fetched text. The host is the authority for native bounds; repeating the
/// public checks here prevents the diagnostic from accepting a malformed peer.
fn validates_fetch_response(value: &JsonValue) -> bool {
    let Some(fields) = value.as_object() else {
        return false;
    };
    if fields.len() != 2 {
        return false;
    }
    let Some(status_code) = fields.get("statusCode").and_then(JsonValue::as_u16) else {
        return false;
    };
    let Some(text) = fields.get("text").and_then(JsonValue::as_string) else {
        return false;
    };
    (100..=599).contains(&status_code) && text.len() <= MAX_NETWORK_TEXT_BYTES
}

#[cfg(test)]
mod tests {
    use anodrel_json::JsonValue;
    use anodrel_network::MAX_NETWORK_TEXT_BYTES;

    use super::{DIAGNOSTIC_URL, fetch_payload, validates_fetch_response};

    #[test]
    fn fetch_payload_names_only_the_compiled_https_url() {
        let payload = fetch_payload();
        let fields = payload.as_object().expect("payload is an object");
        assert_eq!(fields.len(), 1);
        assert_eq!(
            fields.get("url").and_then(JsonValue::as_string),
            Some(DIAGNOSTIC_URL)
        );
    }

    #[test]
    fn accepts_only_the_complete_bounded_public_result() {
        let valid =
            JsonValue::parse(r#"{"statusCode":200,"text":"diagnostic"}"#).expect("fixture is JSON");
        assert!(validates_fetch_response(&valid));

        for malformed in [
            r#"{"statusCode":99,"text":"diagnostic"}"#,
            r#"{"statusCode":600,"text":"diagnostic"}"#,
            r#"{"statusCode":200}"#,
            r#"{"statusCode":200,"text":"diagnostic","header":"forbidden"}"#,
        ] {
            let value = JsonValue::parse(malformed).expect("fixture is JSON");
            assert!(!validates_fetch_response(&value));
        }
        let oversized = JsonValue::parse(&format!(
            r#"{{"statusCode":200,"text":"{}"}}"#,
            "x".repeat(MAX_NETWORK_TEXT_BYTES + 1)
        ))
        .expect("oversized fixture is JSON");
        assert!(!validates_fetch_response(&oversized));
    }
}
