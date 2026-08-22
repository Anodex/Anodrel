//! Validated, portable UI document data.

use std::collections::BTreeSet;

use crate::{ElementId, UiActionTone, UiError, UiSurfaceTone, UiTextTone};

/// The maximum number of nodes in one UI document.
pub const MAX_NODES: usize = 512;
/// The maximum root-inclusive nesting depth in one UI document.
pub const MAX_DEPTH: usize = 32;
/// The maximum combined UTF-8 bytes of text and action labels in one document.
pub const MAX_TEXT_BYTES: usize = 32 * 1024;
/// The smallest supported font size in logical pixels.
pub const MIN_FONT_SIZE: u16 = 8;
/// The largest supported font size in logical pixels.
pub const MAX_FONT_SIZE: u16 = 96;
/// The largest supported padding or gap in logical pixels.
pub const MAX_SPACING: u16 = 256;

/// A stack's primary placement direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Place children from top to bottom.
    Vertical,
    /// Place children from left to right.
    Horizontal,
}

/// Validated padding in logical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Insets {
    pub(crate) left: u16,
    pub(crate) top: u16,
    pub(crate) right: u16,
    pub(crate) bottom: u16,
}

impl Insets {
    /// Builds validated padding values.
    pub fn new(left: u16, top: u16, right: u16, bottom: u16) -> Result<Self, UiError> {
        if [left, top, right, bottom]
            .into_iter()
            .any(|value| value > MAX_SPACING)
        {
            return Err(UiError::InvalidSpacing);
        }
        Ok(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Builds equal padding on every side.
    pub fn all(value: u16) -> Result<Self, UiError> {
        Self::new(value, value, value, value)
    }

    /// Builds zero padding.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    /// Returns left padding.
    #[must_use]
    pub const fn left(self) -> u16 {
        self.left
    }

    /// Returns top padding.
    #[must_use]
    pub const fn top(self) -> u16 {
        self.top
    }

    /// Returns right padding.
    #[must_use]
    pub const fn right(self) -> u16 {
        self.right
    }

    /// Returns bottom padding.
    #[must_use]
    pub const fn bottom(self) -> u16 {
        self.bottom
    }
}

/// A source-ordered stack of child nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stack {
    pub(crate) id: ElementId,
    pub(crate) axis: Axis,
    pub(crate) padding: Insets,
    pub(crate) gap: u16,
    pub(crate) surface_tone: UiSurfaceTone,
    pub(crate) children: Vec<UiNode>,
}

/// One vertical viewport with exactly one scrollable child tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scroll {
    pub(crate) id: ElementId,
    pub(crate) child: Box<UiNode>,
}

impl Scroll {
    /// Builds one scroll viewport around a validated child tree.
    #[must_use]
    pub fn new(id: ElementId, child: UiNode) -> Self {
        Self {
            id,
            child: Box::new(child),
        }
    }

    /// Returns this viewport's stable element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns its one source-ordered child tree.
    #[must_use]
    pub fn child(&self) -> &UiNode {
        &self.child
    }
}

impl Stack {
    /// Builds a stack with bounded padding and inter-child gap.
    pub fn new(
        id: ElementId,
        axis: Axis,
        padding: Insets,
        gap: u16,
        children: Vec<UiNode>,
    ) -> Result<Self, UiError> {
        if gap > MAX_SPACING {
            return Err(UiError::InvalidSpacing);
        }
        Ok(Self {
            id,
            axis,
            padding,
            gap,
            surface_tone: UiSurfaceTone::default(),
            children,
        })
    }

    /// Returns this stack's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns its placement direction.
    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }

    /// Returns its padding.
    #[must_use]
    pub const fn padding(&self) -> Insets {
        self.padding
    }

    /// Returns its inter-child gap.
    #[must_use]
    pub const fn gap(&self) -> u16 {
        self.gap
    }

    /// Returns the requested host-rendered surface treatment.
    #[must_use]
    pub const fn surface_tone(&self) -> UiSurfaceTone {
        self.surface_tone
    }

    /// Requests a host-rendered surface treatment for this stack.
    ///
    /// This is semantic presentation data only. It cannot affect layout,
    /// input, accessibility, or native authority.
    #[must_use]
    pub fn with_surface_tone(mut self, surface_tone: UiSurfaceTone) -> Self {
        self.surface_tone = surface_tone;
        self
    }

    /// Returns child nodes in source order.
    #[must_use]
    pub fn children(&self) -> &[UiNode] {
        &self.children
    }
}

/// A non-interactive, single-line text run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text {
    pub(crate) id: ElementId,
    pub(crate) value: String,
    pub(crate) font_size: u16,
    pub(crate) tone: UiTextTone,
}

impl Text {
    /// Builds validated plain text with a logical-pixel font size.
    pub fn new(id: ElementId, value: impl Into<String>, font_size: u16) -> Result<Self, UiError> {
        let value = value.into();
        validate_text(&value)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            value,
            font_size,
            tone: UiTextTone::default(),
        })
    }

    /// Returns this text run's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated plain-text value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }

    /// Returns the requested host-rendered text prominence.
    #[must_use]
    pub const fn tone(&self) -> UiTextTone {
        self.tone
    }

    /// Requests a host-rendered text prominence for this run.
    ///
    /// The tone does not carry a colour or affect text measurement, layout,
    /// accessibility, input, or native authority.
    #[must_use]
    pub fn with_tone(mut self, tone: UiTextTone) -> Self {
        self.tone = tone;
        self
    }
}

/// The urgency an accessible visible status communicates.
///
/// This is semantic UI data, not a native property or delivery request. A
/// platform adapter may map it to its own assistive-technology vocabulary only
/// after the status is rendered as part of a validated document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiStatusPoliteness {
    /// Let assistive technology speak after its current utterance.
    Polite,
    /// Mark the visible result as urgent to assistive technology.
    Assertive,
}

/// One visible, non-interactive semantic status result.
///
/// A document may contain at most one status. It is ordinary text a host
/// renders, not a hidden announcement or application-selected accessibility
/// property. See `docs/UI_LIVE_ANNOUNCEMENTS.md`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub(crate) id: ElementId,
    pub(crate) value: String,
    pub(crate) font_size: u16,
    pub(crate) tone: UiTextTone,
    pub(crate) politeness: UiStatusPoliteness,
}

impl Status {
    /// Builds one validated visible status with an explicit urgency.
    pub fn new(
        id: ElementId,
        value: impl Into<String>,
        font_size: u16,
        politeness: UiStatusPoliteness,
    ) -> Result<Self, UiError> {
        let value = value.into();
        validate_text(&value)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            value,
            font_size,
            tone: UiTextTone::default(),
            politeness,
        })
    }

    /// Returns this status's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated visible status text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }

    /// Returns the requested host-rendered text prominence.
    #[must_use]
    pub const fn tone(&self) -> UiTextTone {
        self.tone
    }

    /// Returns the semantic urgency of this visible status.
    #[must_use]
    pub const fn politeness(&self) -> UiStatusPoliteness {
        self.politeness
    }

    /// Requests a host-rendered text prominence for this status.
    ///
    /// The tone changes no status semantics, accessibility urgency, native
    /// authority, or event delivery rule.
    #[must_use]
    pub fn with_tone(mut self, tone: UiTextTone) -> Self {
        self.tone = tone;
        self
    }
}

/// A semantic, optionally enabled action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    pub(crate) id: ElementId,
    pub(crate) label: String,
    pub(crate) font_size: u16,
    pub(crate) enabled: bool,
    pub(crate) tone: UiActionTone,
}

impl Action {
    /// Builds a validated semantic action.
    ///
    /// Its [`ElementId`] is the only identity reported by action hit testing;
    /// this type contains no command, callback, or native operation.
    pub fn new(
        id: ElementId,
        label: impl Into<String>,
        font_size: u16,
        enabled: bool,
    ) -> Result<Self, UiError> {
        let label = label.into();
        validate_text(&label)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            label,
            font_size,
            enabled,
            tone: UiActionTone::default(),
        })
    }

    /// Returns this action's semantic element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated visible label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }

    /// Returns whether this action participates in hit testing.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the requested host-rendered action prominence.
    #[must_use]
    pub const fn tone(&self) -> UiActionTone {
        self.tone
    }

    /// Requests a host-rendered action prominence.
    ///
    /// The tone cannot change whether an action is enabled, its semantic ID,
    /// focus order, hit testing, or any native authority.
    #[must_use]
    pub fn with_tone(mut self, tone: UiActionTone) -> Self {
        self.tone = tone;
        self
    }
}

/// Smallest permitted maximum length for a field's text, in characters.
pub const MIN_FIELD_LENGTH: u16 = 1;
/// Largest permitted maximum length for a field's text, in characters.
pub const MAX_FIELD_LENGTH: u16 = 4_096;

/// A single-line field a person can type into.
///
/// This node carries the text the application *starts* the field with. It is
/// not a live value: once a surface is showing the field, the host owns the
/// text, the caret, and the selection, and an application learns a value only
/// by asking for a snapshot through a granted operation. There is no change
/// event and no keystroke anywhere in this model.
///
/// There is deliberately no masked or password variant. See `docs/UI_FIELDS.md`
/// and Decision 0067.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub(crate) id: ElementId,
    pub(crate) label: String,
    pub(crate) value: String,
    pub(crate) placeholder: Option<String>,
    pub(crate) max_length: u16,
    pub(crate) font_size: u16,
    pub(crate) enabled: bool,
}

impl Field {
    /// Builds a validated single-line field.
    ///
    /// `label` is required. A field with no label cannot be announced by a
    /// screen reader, and a control a person cannot identify is not one this
    /// model will produce.
    ///
    /// `value` may be empty — an empty field is the ordinary case — but is
    /// otherwise validated like any visible text, and must fit `max_length`.
    pub fn new(
        id: ElementId,
        label: impl Into<String>,
        value: impl Into<String>,
        max_length: u16,
        font_size: u16,
        enabled: bool,
    ) -> Result<Self, UiError> {
        let label = label.into();
        let value = value.into();
        validate_text(&label)?;
        validate_font_size(font_size)?;
        if !(MIN_FIELD_LENGTH..=MAX_FIELD_LENGTH).contains(&max_length) {
            return Err(UiError::InvalidFieldLength);
        }
        // An empty starting value is ordinary, so it skips the non-empty rule
        // that visible text carries — but not the single-line rule, which is
        // what stops a field from arriving pre-filled with a forged second line.
        if !value.is_empty() {
            validate_text(&value)?;
        }
        if value.chars().count() > usize::from(max_length) {
            return Err(UiError::InvalidText);
        }
        Ok(Self {
            id,
            label,
            value,
            placeholder: None,
            max_length,
            font_size,
            enabled,
        })
    }

    /// Sets the hint shown while the field is empty.
    ///
    /// A placeholder is never returned as a value: it is something the host
    /// draws, not something the person typed.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Result<Self, UiError> {
        let placeholder = placeholder.into();
        validate_text(&placeholder)?;
        self.placeholder = Some(placeholder);
        Ok(self)
    }

    /// Returns this field's semantic element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the validated visible label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the text the application starts this field with.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the hint shown while the field is empty.
    #[must_use]
    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    /// Returns the largest number of characters the host will accept.
    #[must_use]
    pub const fn max_length(&self) -> u16 {
        self.max_length
    }

    /// Returns the requested logical-pixel font size.
    #[must_use]
    pub const fn font_size(&self) -> u16 {
        self.font_size
    }

    /// Returns whether this field accepts focus and input.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// One supported node in a UI document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiNode {
    /// A source-ordered stack.
    Stack(Stack),
    /// A vertically clipped scroll viewport.
    Scroll(Scroll),
    /// A non-interactive text run.
    Text(Text),
    /// One visible semantic status result.
    Status(Status),
    /// A semantic action.
    Action(Action),
    /// A single-line field a person can type into.
    Field(Field),
}

impl UiNode {
    /// Returns this node's validated element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        match self {
            Self::Stack(stack) => stack.id(),
            Self::Scroll(scroll) => scroll.id(),
            Self::Text(text) => text.id(),
            Self::Status(status) => status.id(),
            Self::Action(action) => action.id(),
            Self::Field(field) => field.id(),
        }
    }
}

/// One fully validated in-memory UI tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiDocument {
    root: UiNode,
}

impl UiDocument {
    /// Validates a root node and builds a document.
    ///
    /// This validates document-wide node, depth, text-budget, and unique-ID
    /// limits. It does not parse an application file or accept untrusted input.
    pub fn new(root: UiNode) -> Result<Self, UiError> {
        let mut validator = DocumentValidator::default();
        validator.visit(&root, 1)?;
        Ok(Self { root })
    }

    /// Returns the immutable, validated root node.
    #[must_use]
    pub fn root(&self) -> &UiNode {
        &self.root
    }

    /// Returns this document's one semantic status, when it has one.
    ///
    /// Validation guarantees there is never a second result to choose between.
    #[must_use]
    pub fn status(&self) -> Option<&Status> {
        status_in_node(&self.root)
    }
}

fn status_in_node(node: &UiNode) -> Option<&Status> {
    match node {
        UiNode::Status(status) => Some(status),
        UiNode::Stack(stack) => stack.children.iter().find_map(status_in_node),
        UiNode::Scroll(scroll) => status_in_node(scroll.child()),
        UiNode::Text(_) | UiNode::Action(_) | UiNode::Field(_) => None,
    }
}

fn validate_text(value: &str) -> Result<(), UiError> {
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

fn validate_font_size(font_size: u16) -> Result<(), UiError> {
    if (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&font_size) {
        Ok(())
    } else {
        Err(UiError::InvalidFontSize)
    }
}

#[derive(Default)]
struct DocumentValidator {
    ids: BTreeSet<ElementId>,
    node_count: usize,
    text_bytes: usize,
    status_count: usize,
}

impl DocumentValidator {
    fn visit(&mut self, node: &UiNode, depth: usize) -> Result<(), UiError> {
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

    fn add_text(&mut self, bytes: usize) -> Result<(), UiError> {
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

#[cfg(test)]
mod tests {
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
}
