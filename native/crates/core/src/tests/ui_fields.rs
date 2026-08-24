use super::support::*;
use crate::*;

#[derive(Debug)]
struct FixedFields {
    result: Result<Vec<(&'static str, &'static str)>, UiFieldReadError>,
}

impl UiFieldReader for FixedFields {
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError> {
        let pairs = self.result.as_ref().map_err(|error| *error)?;
        let mut states = anodrel_ui::UiFieldStates::new();
        let children = pairs
            .iter()
            .map(|(id, value)| {
                anodrel_ui::UiNode::Field(
                    anodrel_ui::Field::new(
                        anodrel_ui::ElementId::new(*id).expect("test ID is valid"),
                        "Label",
                        *value,
                        64,
                        14,
                        true,
                    )
                    .expect("test field is valid"),
                )
            })
            .collect();
        let document = anodrel_ui::UiDocument::new(anodrel_ui::UiNode::Stack(
            anodrel_ui::Stack::new(
                anodrel_ui::ElementId::new("root").expect("test ID is valid"),
                anodrel_ui::Axis::Vertical,
                anodrel_ui::Insets::zero(),
                0,
                children,
            )
            .expect("test stack is valid"),
        ))
        .expect("test document is valid");
        states.reseed(&document);
        UiFieldSnapshot::from_states(&states).map_err(|_| UiFieldReadError::Unavailable)
    }
}

fn request_v1_15(operation: &str, payload: &str) -> String {
    format!(
        r#"{{"protocolVersion":{{"major":1,"minor":15}},"kind":"request","requestId":"request-1","operation":"{operation}","payload":{payload}}}"#
    )
}

fn host_with_fields(reader: FixedFields) -> CoreHost {
    CoreHost::with_services(
        HostPolicy::new(
            "test.application",
            vec![Capability::UiFieldsRead],
            "test-host",
        )
        .expect("test policy is valid"),
        HostServices::unavailable().with_ui_fields(reader),
    )
}

#[test]
fn a_granted_field_read_returns_every_value_at_once() {
    let response = JsonValue::parse(
        &host_with_fields(FixedFields {
            result: Ok(vec![("name", "Ada"), ("city", "London")]),
        })
        .handle_json(&request_v1_15("ui.fields.read", "{}")),
    )
    .expect("response JSON is valid");

    assert_eq!(field(&response, "status").as_string(), Some("success"));
    let JsonValue::Array(fields) = field(field(&response, "result"), "fields") else {
        panic!("the result carries an array of fields");
    };
    let pairs: Vec<(Option<&str>, Option<&str>)> = fields
        .iter()
        .map(|entry| {
            (
                field(entry, "id").as_string(),
                field(entry, "value").as_string(),
            )
        })
        .collect();
    // Element-ID order, so the sequence never reports which field was
    // touched last.
    assert_eq!(
        pairs,
        [(Some("city"), Some("London")), (Some("name"), Some("Ada"))]
    );
}

#[test]
fn a_field_read_accepts_no_selector_of_any_kind() {
    // The absence of a selector is the security property: a caller able to
    // narrow a read to one field could repeat it until the typing was
    // reconstructed. See Decision 0067.
    for payload in [
        r#"{"id":"password"}"#,
        r#"{"fields":["password"]}"#,
        r#"{"ids":[]}"#,
        r#"{"since":1}"#,
        r#"{"includeCaret":true}"#,
    ] {
        let response = JsonValue::parse(
            &host_with_fields(FixedFields {
                result: Ok(vec![("name", "Ada")]),
            })
            .handle_json(&request_v1_15("ui.fields.read", payload)),
        )
        .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("request.payload_invalid"),
            "{payload} was accepted"
        );
    }
}

#[test]
fn a_field_read_needs_its_own_grant_and_its_own_protocol_version() {
    let denied = JsonValue::parse(
        &CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiDocumentWrite, Capability::UiEventsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable().with_ui_fields(FixedFields {
                result: Ok(vec![("name", "Ada")]),
            }),
        )
        .handle_json(&request_v1_15("ui.fields.read", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&denied, "error"), "code").as_string(),
        Some("capability.denied")
    );

    // Writing a document does not imply reading what was typed into it.
    let unsupported = JsonValue::parse(
        &host_with_fields(FixedFields {
            result: Ok(vec![("name", "Ada")]),
        })
        .handle_json(&request_v1_14("ui.fields.read", "{}")),
    )
    .expect("response JSON is valid");
    assert_eq!(
        field(field(&unsupported, "error"), "code").as_string(),
        Some("operation.unsupported")
    );
}

#[test]
fn a_host_without_a_surface_reports_one_unavailable_code() {
    for host in [
        host_with_fields(FixedFields {
            result: Err(UiFieldReadError::Unavailable),
        }),
        // No reader supplied at all takes the same path, so an application
        // cannot tell a host without fields from one that refused.
        CoreHost::with_services(
            HostPolicy::new(
                "test.application",
                vec![Capability::UiFieldsRead],
                "test-host",
            )
            .expect("test policy is valid"),
            HostServices::unavailable(),
        ),
    ] {
        let response = JsonValue::parse(&host.handle_json(&request_v1_15("ui.fields.read", "{}")))
            .expect("response JSON is valid");
        assert_eq!(
            field(field(&response, "error"), "code").as_string(),
            Some("ui.fields.unavailable")
        );
    }
}
