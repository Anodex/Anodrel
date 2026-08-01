//! Deterministic encoding of validated UI documents.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;
use anodrel_ui::{
    Action, Axis, Stack, Text, UiActionTone, UiDocument, UiNode, UiSurfaceTone, UiTextTone,
};

use crate::{MAX_ENCODED_DOCUMENT_BYTES, UI_DOCUMENT_FORMAT_V1, UiDocumentError};

/// Encodes a validated document as deterministic version 1 JSON.
///
/// A valid in-memory tree can still exceed this format's fixed encoded limit.
/// In that case no interchange document is returned.
pub fn encode(document: &UiDocument) -> Result<String, UiDocumentError> {
    let mut fields = BTreeMap::new();
    fields.insert(
        "format".to_owned(),
        JsonValue::String(UI_DOCUMENT_FORMAT_V1.to_owned()),
    );
    fields.insert("root".to_owned(), node(document.root())?);
    let encoded = JsonValue::Object(fields).to_json();
    if encoded.len() > MAX_ENCODED_DOCUMENT_BYTES {
        Err(UiDocumentError::EncodedLimitExceeded)
    } else {
        Ok(encoded)
    }
}

fn node(node: &UiNode) -> Result<JsonValue, UiDocumentError> {
    match node {
        UiNode::Stack(stack) => stack_value(stack),
        UiNode::Scroll(_) => Err(UiDocumentError::UnsupportedFormat),
        UiNode::Text(text) => Ok(text_value(text)),
        UiNode::Action(action) => Ok(action_value(action)),
    }
}

fn stack_value(stack: &Stack) -> Result<JsonValue, UiDocumentError> {
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
                .map(node)
                .collect::<Result<Vec<_>, _>>()?,
        ),
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
