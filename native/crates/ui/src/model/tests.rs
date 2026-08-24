//! Focused verification for validated native UI documents.

use super::*;

fn id(value: impl Into<String>) -> ElementId {
    ElementId::new(value).expect("test ID is valid")
}

fn text(id_value: impl Into<String>, value: impl Into<String>) -> UiNode {
    UiNode::Text(Text::new(id(id_value), value, 10).expect("test text is valid"))
}

fn stack(id_value: impl Into<String>, child: UiNode) -> UiNode {
    UiNode::Stack(
        Stack::new(id(id_value), Axis::Vertical, Insets::zero(), 0, vec![child])
            .expect("test stack is valid"),
    )
}

fn field(id_value: &str, value: &str) -> Field {
    Field::new(id(id_value), "Name", value, 64, 14, true).expect("test field is valid")
}

#[test]
fn a_field_accepts_an_empty_starting_value_but_not_a_forged_second_line() {
    // Empty is the ordinary case, so a field skips the non-empty rule that
    // visible text carries. It does not skip the single-line rule: a value
    // arriving with a newline could present one field as two.
    assert!(Field::new(id("empty"), "Name", "", 64, 14, true).is_ok());
    assert_eq!(
        Field::new(id("forged"), "Name", "Alice\nBob", 64, 14, true),
        Err(UiError::InvalidText)
    );
    assert_eq!(
        Field::new(id("escape"), "Name", "Alice\u{1B}[2K", 64, 14, true),
        Err(UiError::InvalidText)
    );
}

#[test]
fn a_field_requires_a_label_because_an_unnamed_control_cannot_be_announced() {
    assert_eq!(
        Field::new(id("nameless"), "", "", 64, 14, true),
        Err(UiError::InvalidText)
    );
}

#[test]
fn a_field_bounds_its_maximum_length_and_its_starting_value() {
    assert_eq!(
        Field::new(id("zero"), "Name", "", MIN_FIELD_LENGTH - 1, 14, true),
        Err(UiError::InvalidFieldLength)
    );
    assert_eq!(
        Field::new(id("huge"), "Name", "", MAX_FIELD_LENGTH + 1, 14, true),
        Err(UiError::InvalidFieldLength)
    );
    assert!(Field::new(id("edge"), "Name", "", MAX_FIELD_LENGTH, 14, true).is_ok());

    // A starting value longer than the field will accept would put the host
    // in a state a person could not have typed into.
    assert!(Field::new(id("fits"), "Name", "abcd", 4, 14, true).is_ok());
    assert_eq!(
        Field::new(id("over"), "Name", "abcde", 4, 14, true),
        Err(UiError::InvalidText)
    );
}

#[test]
fn a_field_measures_its_starting_value_in_characters_not_bytes() {
    // The limit is what a person may type, so it counts what they type.
    // Four emoji are four characters and sixteen bytes.
    let emoji = "\u{1F680}".repeat(4);
    assert_eq!(emoji.chars().count(), 4);
    assert_eq!(emoji.len(), 16);
    assert!(Field::new(id("emoji"), "Name", emoji.clone(), 4, 14, true).is_ok());
    assert_eq!(
        Field::new(id("emoji-over"), "Name", emoji, 3, 14, true),
        Err(UiError::InvalidText)
    );
}

#[test]
fn every_string_a_field_carries_counts_towards_the_document_budget() {
    // A document of pre-filled fields is as large as one of text runs, so
    // the value and the placeholder are budgeted like any other text.
    // Each value is within one field's own limit; it takes several of them
    // to exceed the document's. That is exactly the case a per-field bound
    // alone would miss.
    let filler = "x".repeat(usize::from(MAX_FIELD_LENGTH));
    let needed = MAX_TEXT_BYTES / usize::from(MAX_FIELD_LENGTH) + 1;
    let children = (0..needed)
        .map(|index| {
            UiNode::Field(
                Field::new(
                    id(format!("field-{index}")),
                    "Name",
                    filler.clone(),
                    MAX_FIELD_LENGTH,
                    14,
                    true,
                )
                .expect("each value fits its own field"),
            )
        })
        .collect();
    let root = UiNode::Stack(
        Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
            .expect("test stack is valid"),
    );
    assert_eq!(UiDocument::new(root), Err(UiError::TextLimitExceeded));
}

#[test]
fn a_placeholder_is_validated_and_kept_separate_from_the_value() {
    let with_hint = field("hinted", "")
        .with_placeholder("Your full name")
        .expect("placeholder is valid");
    assert_eq!(with_hint.placeholder(), Some("Your full name"));
    // A placeholder is drawn, never returned as a value.
    assert_eq!(with_hint.value(), "");
    assert_eq!(field("plain", "").placeholder(), None);
    assert_eq!(
        field("bad", "").with_placeholder("two\nlines").unwrap_err(),
        UiError::InvalidText
    );
}

#[test]
fn rejects_a_document_deeper_than_the_limit() {
    let mut node = text("leaf", "content");
    for index in 0..MAX_DEPTH {
        node = stack(format!("stack-{index}"), node);
    }
    assert_eq!(UiDocument::new(node), Err(UiError::DepthLimitExceeded));
}

#[test]
fn rejects_a_document_with_more_than_the_node_limit() {
    let children = (0..MAX_NODES)
        .map(|index| text(format!("item-{index}"), "x"))
        .collect();
    let root = UiNode::Stack(
        Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
            .expect("test stack is valid"),
    );
    assert_eq!(UiDocument::new(root), Err(UiError::NodeLimitExceeded));
}

#[test]
fn rejects_text_that_exceeds_the_document_budget() {
    let root = UiNode::Stack(
        Stack::new(
            id("root"),
            Axis::Vertical,
            Insets::zero(),
            0,
            vec![
                text("full", "x".repeat(MAX_TEXT_BYTES)),
                text("one-more", "x"),
            ],
        )
        .expect("test stack is valid"),
    );
    assert_eq!(UiDocument::new(root), Err(UiError::TextLimitExceeded));
}

#[test]
fn appearance_roles_default_and_can_be_selected_without_changing_content() {
    let stack = Stack::new(id("stack"), Axis::Vertical, Insets::zero(), 0, vec![])
        .expect("test stack is valid");
    let text = Text::new(id("text"), "Content", 10).expect("test text is valid");
    let action = Action::new(id("action"), "Continue", 10, true).expect("test action is valid");

    assert_eq!(stack.surface_tone(), UiSurfaceTone::Plain);
    assert_eq!(text.tone(), UiTextTone::Primary);
    assert_eq!(action.tone(), UiActionTone::Neutral);

    assert_eq!(
        stack
            .with_surface_tone(UiSurfaceTone::Raised)
            .surface_tone(),
        UiSurfaceTone::Raised
    );
    assert_eq!(
        text.with_tone(UiTextTone::Accent).tone(),
        UiTextTone::Accent
    );
    assert_eq!(
        action.with_tone(UiActionTone::Accent).tone(),
        UiActionTone::Accent
    );
}

#[test]
fn one_visible_status_is_preserved_but_a_second_is_rejected() {
    let first = Status::new(id("status"), "Saved", 14, UiStatusPoliteness::Polite)
        .expect("status is valid")
        .with_tone(UiTextTone::Accent);
    let document = UiDocument::new(UiNode::Status(first.clone())).expect("one status is valid");
    assert_eq!(document.status(), Some(&first));
    assert_eq!(first.tone(), UiTextTone::Accent);

    let second = Status::new(
        id("other-status"),
        "Synced",
        14,
        UiStatusPoliteness::Assertive,
    )
    .expect("second status is valid");
    let tree = UiNode::Stack(
        Stack::new(
            id("root"),
            Axis::Vertical,
            Insets::zero(),
            0,
            vec![UiNode::Status(first), UiNode::Status(second)],
        )
        .expect("stack is valid"),
    );
    assert_eq!(UiDocument::new(tree), Err(UiError::StatusLimitExceeded));
}
