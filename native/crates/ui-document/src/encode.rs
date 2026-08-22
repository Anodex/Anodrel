//! Deterministic encoding of validated UI documents.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;
use anodrel_ui::{
    Action, Axis, Field, Scroll, Stack, Status, Text, UiActionTone, UiDocument, UiNode,
    UiStatusPoliteness, UiSurfaceTone, UiTextTone,
};

use crate::{
    MAX_ENCODED_DOCUMENT_BYTES, UI_DOCUMENT_FORMAT_V1, UI_DOCUMENT_FORMAT_V2,
    UI_DOCUMENT_FORMAT_V3, UiDocumentError,
};

/// Encodes a validated document as deterministic version 1 JSON.
///
/// A valid in-memory tree can still exceed this format's fixed encoded limit.
/// In that case no interchange document is returned.
pub fn encode(document: &UiDocument) -> Result<String, UiDocumentError> {
    encode_format(document, UI_DOCUMENT_FORMAT_V1, false, false)
}

/// Encodes a validated document as deterministic version 2 JSON.
pub fn encode_v2(document: &UiDocument) -> Result<String, UiDocumentError> {
    encode_format(document, UI_DOCUMENT_FORMAT_V2, true, false)
}

/// Encodes a validated document as deterministic version 3 JSON.
pub fn encode_v3(document: &UiDocument) -> Result<String, UiDocumentError> {
    encode_format(document, UI_DOCUMENT_FORMAT_V3, true, true)
}

fn encode_format(
    document: &UiDocument,
    format: &str,
    allows_scroll: bool,
    allows_status: bool,
) -> Result<String, UiDocumentError> {
    let mut fields = BTreeMap::new();
    fields.insert("format".to_owned(), JsonValue::String(format.to_owned()));
    fields.insert(
        "root".to_owned(),
        node(document.root(), allows_scroll, allows_status)?,
    );
    let encoded = JsonValue::Object(fields).to_json();
    if encoded.len() > MAX_ENCODED_DOCUMENT_BYTES {
        Err(UiDocumentError::EncodedLimitExceeded)
    } else {
        Ok(encoded)
    }
}

fn node(
    node: &UiNode,
    allows_scroll: bool,
    allows_status: bool,
) -> Result<JsonValue, UiDocumentError> {
    match node {
        UiNode::Stack(stack) => stack_value(stack, allows_scroll, allows_status),
        UiNode::Scroll(scroll) if allows_scroll => {
            scroll_value(scroll, allows_scroll, allows_status)
        }
        UiNode::Scroll(_) => Err(UiDocumentError::UnsupportedFormat),
        UiNode::Text(text) => Ok(text_value(text)),
        UiNode::Status(status) if allows_status => Ok(status_value(status)),
        UiNode::Status(_) => Err(UiDocumentError::UnsupportedFormat),
        UiNode::Action(action) => Ok(action_value(action)),
        UiNode::Field(field) => Ok(field_value(field)),
    }
}

fn stack_value(
    stack: &Stack,
    allows_scroll: bool,
    allows_status: bool,
) -> Result<JsonValue, UiDocumentError> {
    let mut fields = common_fields(stack.id().as_str(), "stack");
    fields.insert(
        "axis".to_owned(),
        JsonValue::String(
            match stack.axis() {
                Axis::Vertical => "vertical",
                Axis::Horizontal => "horizontal",
            }
            .to_owned(),
        ),
    );
    fields.insert("padding".to_owned(), padding_value(stack));
    fields.insert("gap".to_owned(), number(stack.gap()));
    fields.insert(
        "surfaceTone".to_owned(),
        JsonValue::String(
            match stack.surface_tone() {
                UiSurfaceTone::Plain => "plain",
                UiSurfaceTone::Raised => "raised",
            }
            .to_owned(),
        ),
    );
    fields.insert(
        "children".to_owned(),
        JsonValue::Array(
            stack
                .children()
                .iter()
                .map(|child| node(child, allows_scroll, allows_status))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    Ok(JsonValue::Object(fields))
}

fn scroll_value(
    scroll: &Scroll,
    allows_scroll: bool,
    allows_status: bool,
) -> Result<JsonValue, UiDocumentError> {
    let mut fields = common_fields(scroll.id().as_str(), "scroll");
    fields.insert(
        "child".to_owned(),
        node(scroll.child(), allows_scroll, allows_status)?,
    );
    Ok(JsonValue::Object(fields))
}

fn text_value(text: &Text) -> JsonValue {
    let mut fields = common_fields(text.id().as_str(), "text");
    fields.insert(
        "value".to_owned(),
        JsonValue::String(text.value().to_owned()),
    );
    fields.insert("fontSize".to_owned(), number(text.font_size()));
    fields.insert(
        "tone".to_owned(),
        JsonValue::String(
            match text.tone() {
                UiTextTone::Primary => "primary",
                UiTextTone::Secondary => "secondary",
                UiTextTone::Accent => "accent",
            }
            .to_owned(),
        ),
    );
    JsonValue::Object(fields)
}

fn status_value(status: &Status) -> JsonValue {
    let mut fields = common_fields(status.id().as_str(), "status");
    fields.insert(
        "value".to_owned(),
        JsonValue::String(status.value().to_owned()),
    );
    fields.insert("fontSize".to_owned(), number(status.font_size()));
    fields.insert(
        "tone".to_owned(),
        JsonValue::String(
            match status.tone() {
                UiTextTone::Primary => "primary",
                UiTextTone::Secondary => "secondary",
                UiTextTone::Accent => "accent",
            }
            .to_owned(),
        ),
    );
    fields.insert(
        "politeness".to_owned(),
        JsonValue::String(
            match status.politeness() {
                UiStatusPoliteness::Polite => "polite",
                UiStatusPoliteness::Assertive => "assertive",
            }
            .to_owned(),
        ),
    );
    JsonValue::Object(fields)
}

fn action_value(action: &Action) -> JsonValue {
    let mut fields = common_fields(action.id().as_str(), "action");
    fields.insert(
        "label".to_owned(),
        JsonValue::String(action.label().to_owned()),
    );
    fields.insert("fontSize".to_owned(), number(action.font_size()));
    fields.insert("enabled".to_owned(), JsonValue::Bool(action.enabled()));
    fields.insert(
        "tone".to_owned(),
        JsonValue::String(
            match action.tone() {
                UiActionTone::Neutral => "neutral",
                UiActionTone::Accent => "accent",
            }
            .to_owned(),
        ),
    );
    JsonValue::Object(fields)
}

/// Encodes a field, including the value the application starts it with.
///
/// This direction carries a value because a document *sets* one. Nothing here
/// reads a value back out of a live surface: the host's current text never
/// becomes a document. See `docs/UI_FIELDS.md`.
fn field_value(field: &Field) -> JsonValue {
    let mut fields = common_fields(field.id().as_str(), "field");
    fields.insert(
        "label".to_owned(),
        JsonValue::String(field.label().to_owned()),
    );
    fields.insert(
        "value".to_owned(),
        JsonValue::String(field.value().to_owned()),
    );
    if let Some(placeholder) = field.placeholder() {
        fields.insert(
            "placeholder".to_owned(),
            JsonValue::String(placeholder.to_owned()),
        );
    }
    fields.insert("maxLength".to_owned(), number(field.max_length()));
    fields.insert("fontSize".to_owned(), number(field.font_size()));
    fields.insert("enabled".to_owned(), JsonValue::Bool(field.enabled()));
    JsonValue::Object(fields)
}

fn common_fields(id: &str, kind: &str) -> BTreeMap<String, JsonValue> {
    BTreeMap::from([
        ("id".to_owned(), JsonValue::String(id.to_owned())),
        ("kind".to_owned(), JsonValue::String(kind.to_owned())),
    ])
}

fn padding_value(stack: &Stack) -> JsonValue {
    let padding = stack.padding();
    JsonValue::Object(BTreeMap::from([
        ("bottom".to_owned(), number(padding.bottom())),
        ("left".to_owned(), number(padding.left())),
        ("right".to_owned(), number(padding.right())),
        ("top".to_owned(), number(padding.top())),
    ]))
}

fn number(value: u16) -> JsonValue {
    JsonValue::Number(value.to_string())
}
