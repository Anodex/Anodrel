//! Strict decoding of the version 1 UI document schema.

use std::collections::BTreeMap;

use anodrel_json::JsonValue;
use anodrel_ui::{
    Action, Axis, ElementId, Insets, Stack, Text, UiActionTone, UiDocument, UiNode, UiSurfaceTone,
    UiTextTone,
};

use crate::{MAX_ENCODED_DOCUMENT_BYTES, UI_DOCUMENT_FORMAT_V1, UiDocumentError};

/// Decodes one exact, bounded version 1 UI document.
pub fn decode(input: &str) -> Result<UiDocument, UiDocumentError> {
    if input.len() > MAX_ENCODED_DOCUMENT_BYTES {
        return Err(UiDocumentError::EncodedLimitExceeded);
    }
    let value = JsonValue::parse(input).map_err(|_| UiDocumentError::InvalidJson)?;
    let fields = object(&value)?;
    require_fields(fields, &["format", "root"])?;
    if string_field(fields, "format")? != UI_DOCUMENT_FORMAT_V1 {
        return Err(UiDocumentError::UnsupportedFormat);
    }
    UiDocument::new(node(required_field(fields, "root")?)?).map_err(Into::into)
}

fn node(value: &JsonValue) -> Result<UiNode, UiDocumentError> {
    let fields = object(value)?;
    match string_field(fields, "kind")? {
        "stack" => stack(fields),
        "text" => text(fields),
        "action" => action(fields),
        _ => Err(UiDocumentError::UnsupportedNodeKind),
    }
}

fn stack(fields: &BTreeMap<String, JsonValue>) -> Result<UiNode, UiDocumentError> {
    require_fields(
        fields,
        &[
            "id",
            "kind",
            "axis",
            "padding",
            "gap",
            "surfaceTone",
            "children",
        ],
    )?;
    let axis = match string_field(fields, "axis")? {
        "vertical" => Axis::Vertical,
        "horizontal" => Axis::Horizontal,
        _ => return Err(UiDocumentError::InvalidField),
    };
    let surface_tone = match string_field(fields, "surfaceTone")? {
        "plain" => UiSurfaceTone::Plain,
        "raised" => UiSurfaceTone::Raised,
        _ => return Err(UiDocumentError::InvalidField),
    };
    let children = array(required_field(fields, "children")?)?
        .iter()
        .map(node)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UiNode::Stack(
        Stack::new(
            element_id(fields)?,
            axis,
            padding(required_field(fields, "padding")?)?,
            u16_field(fields, "gap")?,
            children,
        )?
        .with_surface_tone(surface_tone),
    ))
}

fn text(fields: &BTreeMap<String, JsonValue>) -> Result<UiNode, UiDocumentError> {
    require_fields(fields, &["id", "kind", "value", "fontSize", "tone"])?;
    let tone = match string_field(fields, "tone")? {
        "primary" => UiTextTone::Primary,
        "secondary" => UiTextTone::Secondary,
        "accent" => UiTextTone::Accent,
        _ => return Err(UiDocumentError::InvalidField),
    };
    Ok(UiNode::Text(
        Text::new(
            element_id(fields)?,
            string_field(fields, "value")?,
            u16_field(fields, "fontSize")?,
        )?
        .with_tone(tone),
    ))
}

fn action(fields: &BTreeMap<String, JsonValue>) -> Result<UiNode, UiDocumentError> {
    require_fields(
        fields,
        &["id", "kind", "label", "fontSize", "enabled", "tone"],
    )?;
    let tone = match string_field(fields, "tone")? {
        "neutral" => UiActionTone::Neutral,
        "accent" => UiActionTone::Accent,
        _ => return Err(UiDocumentError::InvalidField),
    };
    Ok(UiNode::Action(
        Action::new(
            element_id(fields)?,
            string_field(fields, "label")?,
            u16_field(fields, "fontSize")?,
            bool_field(fields, "enabled")?,
        )?
        .with_tone(tone),
    ))
}

fn padding(value: &JsonValue) -> Result<Insets, UiDocumentError> {
    let fields = object(value)?;
    require_fields(fields, &["left", "top", "right", "bottom"])?;
    Insets::new(
        u16_field(fields, "left")?,
        u16_field(fields, "top")?,
        u16_field(fields, "right")?,
        u16_field(fields, "bottom")?,
    )
    .map_err(Into::into)
}

fn element_id(fields: &BTreeMap<String, JsonValue>) -> Result<ElementId, UiDocumentError> {
    ElementId::new(string_field(fields, "id")?).map_err(Into::into)
}

fn object(value: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, UiDocumentError> {
    value.as_object().ok_or(UiDocumentError::ExpectedObject)
}

fn array(value: &JsonValue) -> Result<&[JsonValue], UiDocumentError> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(UiDocumentError::ExpectedArray),
    }
}

fn require_fields(
    fields: &BTreeMap<String, JsonValue>,
    allowed: &[&str],
) -> Result<(), UiDocumentError> {
    if fields.len() != allowed.len() {
        return if fields
            .keys()
            .any(|field| !allowed.contains(&field.as_str()))
        {
            Err(UiDocumentError::UnknownField)
        } else {
            Err(UiDocumentError::MissingField)
        };
    }
    if fields
        .keys()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(UiDocumentError::UnknownField);
    }
    if allowed.iter().any(|field| !fields.contains_key(*field)) {
        return Err(UiDocumentError::MissingField);
    }
    Ok(())
}

fn required_field<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a JsonValue, UiDocumentError> {
    fields.get(name).ok_or(UiDocumentError::MissingField)
}

fn string_field<'a>(
    fields: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, UiDocumentError> {
    required_field(fields, name)?
        .as_string()
        .ok_or(UiDocumentError::InvalidField)
}

fn u16_field(fields: &BTreeMap<String, JsonValue>, name: &str) -> Result<u16, UiDocumentError> {
    required_field(fields, name)?
        .as_u16()
        .ok_or(UiDocumentError::InvalidField)
}

fn bool_field(fields: &BTreeMap<String, JsonValue>, name: &str) -> Result<bool, UiDocumentError> {
    match required_field(fields, name)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(UiDocumentError::InvalidField),
    }
}
