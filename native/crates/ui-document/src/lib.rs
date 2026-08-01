//! Strict, capability-free interchange for Anodrel's native UI document.
//!
//! This crate decodes and encodes only `anodrel.ui.document.v1`, a bounded JSON
//! form of [`anodrel_ui::UiDocument`]. It has no renderer, operating-system
//! call, package loader, protocol operation, session, callback, or capability.
//! A future consumer must establish those boundaries separately.
//!
//! See `docs/UI_DOCUMENTS.md` and Decision 0029 for the public contract.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod decode;
mod encode;
mod error;

pub use decode::decode;
pub use encode::encode;
pub use error::UiDocumentError;

/// The only accepted version 1 external UI document format identifier.
pub const UI_DOCUMENT_FORMAT_V1: &str = "anodrel.ui.document.v1";
/// The largest accepted UTF-8 JSON document in bytes.
pub const MAX_ENCODED_DOCUMENT_BYTES: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use anodrel_ui::{
        Action, Axis, ElementId, Insets, Stack, Text, UiActionTone, UiDocument, UiError, UiNode,
        UiSurfaceTone, UiTextTone,
    };

    use super::{MAX_ENCODED_DOCUMENT_BYTES, UiDocumentError, decode, encode};

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn document() -> UiDocument {
        UiDocument::new(UiNode::Stack(
            Stack::new(
                id("root"),
                Axis::Vertical,
                Insets::new(2, 4, 6, 8).expect("test padding is valid"),
                10,
                vec![
                    UiNode::Text(
                        Text::new(id("title"), "Welcome", 24)
                            .expect("test text is valid")
                            .with_tone(UiTextTone::Accent),
                    ),
                    UiNode::Action(
                        Action::new(id("continue"), "Continue", 16, true)
                            .expect("test action is valid")
                            .with_tone(UiActionTone::Accent),
                    ),
                ],
            )
            .expect("test stack is valid")
            .with_surface_tone(UiSurfaceTone::Raised),
        ))
        .expect("test document is valid")
    }

    #[test]
    fn round_trips_every_node_type_and_appearance_role() {
        let document = document();
        let encoded = encode(&document).expect("test document fits the format limit");

        assert_eq!(decode(&encoded), Ok(document.clone()));
        assert_eq!(encode(&document), Ok(encoded));
    }

    #[test]
    fn rejects_schema_extensions_and_missing_required_fields() {
        assert_eq!(
            decode(r#"{"format":"anodrel.ui.document.v1","root":{},"extra":true}"#),
            Err(UiDocumentError::UnknownField)
        );
        assert_eq!(
            decode(r#"{"format":"anodrel.ui.document.v1"}"#),
            Err(UiDocumentError::MissingField)
        );
        assert_eq!(
            decode(
                r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"One","fontSize":12}}"#
            ),
            Err(UiDocumentError::MissingField)
        );
    }

    #[test]
    fn rejects_unsupported_or_invalid_input_without_echoing_it() {
        assert_eq!(
            decode(r#"{"format":"anodrel.ui.document.v2","root":{}}"#),
            Err(UiDocumentError::UnsupportedFormat)
        );
        assert_eq!(
            decode(r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"button"}}"#),
            Err(UiDocumentError::UnsupportedNodeKind)
        );
        assert_eq!(decode("not JSON"), Err(UiDocumentError::InvalidJson));
        assert_eq!(
            decode(
                r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"One","fontSize":12.5,"tone":"primary"}}"#
            ),
            Err(UiDocumentError::InvalidField)
        );
    }

    #[test]
    fn applies_encoded_and_model_bounds() {
        assert_eq!(
            decode(&" ".repeat(MAX_ENCODED_DOCUMENT_BYTES + 1)),
            Err(UiDocumentError::EncodedLimitExceeded)
        );
        assert_eq!(
            decode(
                r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"One","fontSize":7,"tone":"primary"}}"#
            ),
            Err(UiDocumentError::InvalidModel(UiError::InvalidFontSize))
        );
    }

    #[test]
    fn does_not_encode_a_valid_model_past_the_interchange_limit() {
        let children = (0..511)
            .map(|index| {
                UiNode::Text(
                    Text::new(id(&format!("item-{index:059}")), "x".repeat(64), 8)
                        .expect("test text is valid"),
                )
            })
            .collect();
        let document = UiDocument::new(UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        ))
        .expect("the model remains inside its own limits");

        assert_eq!(
            encode(&document),
            Err(UiDocumentError::EncodedLimitExceeded)
        );
    }
}
