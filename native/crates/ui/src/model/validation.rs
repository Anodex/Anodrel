//! Structural validation for one bounded native UI document.

use std::collections::BTreeSet;

use super::*;

pub(super) fn status_in_node(node: &UiNode) -> Option<&Status> {
    match node {
        UiNode::Status(status) => Some(status),
        UiNode::Stack(stack) => stack.children.iter().find_map(status_in_node),
        UiNode::Scroll(scroll) => status_in_node(scroll.child()),
        UiNode::Text(_) | UiNode::Action(_) | UiNode::Field(_) => None,
    }
}

pub(super) fn validate_text(value: &str) -> Result<(), UiError> {
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        Err(UiError::InvalidText)
    } else {
        Ok(())
    }
}

pub(super) fn validate_font_size(font_size: u16) -> Result<(), UiError> {
    if (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&font_size) {
        Ok(())
    } else {
        Err(UiError::InvalidFontSize)
    }
}

#[derive(Default)]
pub(super) struct DocumentValidator {
    ids: BTreeSet<ElementId>,
    node_count: usize,
    text_bytes: usize,
    status_count: usize,
}

impl DocumentValidator {
    pub(super) fn visit(&mut self, node: &UiNode, depth: usize) -> Result<(), UiError> {
        if depth > MAX_DEPTH {
            return Err(UiError::DepthLimitExceeded);
        }
        self.node_count += 1;
        if self.node_count > MAX_NODES {
            return Err(UiError::NodeLimitExceeded);
        }
        if !self.ids.insert(node.id().clone()) {
            return Err(UiError::DuplicateElementId);
        }

        match node {
            UiNode::Stack(stack) => {
                for child in &stack.children {
                    self.visit(child, depth + 1)?;
                }
            }
            UiNode::Scroll(scroll) => self.visit(scroll.child(), depth + 1)?,
            UiNode::Text(text) => self.add_text(text.value.len())?,
            UiNode::Status(status) => {
                self.status_count += 1;
                if self.status_count > 1 {
                    return Err(UiError::StatusLimitExceeded);
                }
                self.add_text(status.value.len())?;
            }
            UiNode::Action(action) => self.add_text(action.label.len())?,
            // Every string a field carries counts towards the document budget,
            // including its starting value: a document that arrives with 512
            // pre-filled fields is as large as one with 512 text runs.
            UiNode::Field(field) => self.add_text(
                field.label.len()
                    + field.value.len()
                    + field.placeholder.as_ref().map_or(0, String::len),
            )?,
        }
        Ok(())
    }

    pub(super) fn add_text(&mut self, bytes: usize) -> Result<(), UiError> {
        self.text_bytes = self
            .text_bytes
            .checked_add(bytes)
            .ok_or(UiError::TextLimitExceeded)?;
        if self.text_bytes > MAX_TEXT_BYTES {
            Err(UiError::TextLimitExceeded)
        } else {
            Ok(())
        }
    }
}
