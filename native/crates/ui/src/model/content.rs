//! Leaf content nodes for the validated UI document tree.

use super::*;

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
