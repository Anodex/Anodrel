//! The bounded snapshot of field values one session may read.
//!
//! This is the only path by which what a person typed reaches an application,
//! and it carries the values and nothing else. See `docs/UI_FIELDS.md` and
//! Decision 0067.

use std::fmt;

use anodrel_ui::{ElementId, MAX_FIELD_LENGTH, UiFieldStates};

/// Largest number of fields one snapshot may carry.
///
/// A document is already bounded to 512 nodes, so this cannot be reached by a
/// document the host accepted. It exists so the response size has a stated
/// ceiling rather than one inherited from somewhere else.
pub const MAX_SNAPSHOT_FIELDS: usize = 64;

/// One field's identity and current text.
///
/// Deliberately two values. A caret, a selection, a character count, a
/// timestamp, or an edited flag would each describe the *typing* rather than
/// the value, which is the distinction the whole contract rests on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiFieldValue {
    id: ElementId,
    value: String,
}

impl UiFieldValue {
    /// Returns the field's element ID.
    #[must_use]
    pub fn id(&self) -> &ElementId {
        &self.id
    }

    /// Returns the field's current text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Every field on one surface, as it stands at the moment of a read.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiFieldSnapshot {
    fields: Vec<UiFieldValue>,
}

impl UiFieldSnapshot {
    /// Builds a snapshot from the host's current field state.
    ///
    /// Fields arrive in element-ID order because the state is keyed that way.
    /// A stable order matters: one that varied with focus or edit history would
    /// leak which field was touched last, through the sequence rather than
    /// through a field.
    ///
    /// # Errors
    ///
    /// Returns [`UiFieldSnapshotError::TooManyFields`] beyond
    /// [`MAX_SNAPSHOT_FIELDS`], and [`UiFieldSnapshotError::ValueTooLarge`] for
    /// a value beyond the model's own per-field bound — which a host's own
    /// state should never exceed, so reaching it means something upstream is
    /// wrong and the read fails rather than truncating.
    pub fn from_states(states: &UiFieldStates) -> Result<Self, UiFieldSnapshotError> {
        if states.len() > MAX_SNAPSHOT_FIELDS {
            return Err(UiFieldSnapshotError::TooManyFields);
        }
        let mut fields = Vec::with_capacity(states.len());
        for (id, state) in states.iter() {
            if state.character_count() > usize::from(MAX_FIELD_LENGTH) {
                return Err(UiFieldSnapshotError::ValueTooLarge);
            }
            fields.push(UiFieldValue {
                id: id.clone(),
                value: state.text().to_owned(),
            });
        }
        Ok(Self { fields })
    }

    /// Returns the field values in element-ID order.
    #[must_use]
    pub fn fields(&self) -> &[UiFieldValue] {
        &self.fields
    }
}

/// The portable service boundary a host core reads field values through.
///
/// An implementation must answer only for the requesting session's own current
/// surface. It must not accept a selector, because one would let a caller
/// narrow a read and repeat it until the typing was reconstructed.
pub trait UiFieldReader: fmt::Debug + Send {
    /// Returns every field on the current surface.
    ///
    /// # Errors
    ///
    /// Returns [`UiFieldReadError`] when this session has no surface to read,
    /// or the host could not answer in time.
    fn read(&self) -> Result<UiFieldSnapshot, UiFieldReadError>;
}

/// A safe failure category for a field read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiFieldReadError {
    /// This session has no surface with fields, or the host could not answer.
    ///
    /// One category deliberately. Distinguishing "no surface" from "no fields"
    /// from "the UI thread was busy" would report host state, and repeated
    /// reads of that state describe what the person is doing.
    Unavailable,
}

impl fmt::Display for UiFieldReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("no field values are available")
    }
}

impl std::error::Error for UiFieldReadError {}

/// Why a snapshot could not be built from host state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiFieldSnapshotError {
    /// More fields than [`MAX_SNAPSHOT_FIELDS`].
    TooManyFields,
    /// A value beyond the model's per-field bound.
    ValueTooLarge,
}

impl fmt::Display for UiFieldSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooManyFields => "surface carries more fields than one snapshot may report",
            Self::ValueTooLarge => "a field value exceeds the model's per-field bound",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for UiFieldSnapshotError {}

#[cfg(test)]
mod tests {
    use super::{MAX_SNAPSHOT_FIELDS, UiFieldSnapshot, UiFieldSnapshotError};
    use anodrel_ui::{Axis, ElementId, Field, Insets, Stack, UiDocument, UiFieldStates, UiNode};

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn states_for(names: &[&str]) -> UiFieldStates {
        let children = names
            .iter()
            .map(|name| {
                UiNode::Field(
                    Field::new(id(name), "Label", *name, 64, 14, true)
                        .expect("test field is valid"),
                )
            })
            .collect();
        let document = UiDocument::new(UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        ))
        .expect("test document is valid");
        let mut states = UiFieldStates::new();
        states.reseed(&document);
        states
    }

    #[test]
    fn a_snapshot_carries_every_field_in_a_stable_order() {
        // Order comes from the element ID, not from focus or edit history. An
        // order that varied with either would report which field was touched
        // last through the sequence rather than through a field.
        let snapshot = UiFieldSnapshot::from_states(&states_for(&["gamma", "alpha", "beta"]))
            .expect("the snapshot builds");
        let ids: Vec<&str> = snapshot
            .fields()
            .iter()
            .map(|field| field.id().as_str())
            .collect();
        assert_eq!(ids, ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn a_snapshot_carries_the_value_and_nothing_that_describes_the_typing() {
        let snapshot =
            UiFieldSnapshot::from_states(&states_for(&["name"])).expect("the snapshot builds");
        let field = &snapshot.fields()[0];
        assert_eq!(field.id().as_str(), "name");
        assert_eq!(field.value(), "name");
        // The type has exactly two accessors. This is the guard against a
        // caret, a timestamp, or an edited flag being added without anyone
        // revisiting Decision 0067.
        let debug = format!("{field:?}");
        for absent in ["caret", "selection", "edited", "focused", "timestamp"] {
            assert!(!debug.contains(absent), "{absent} reached a field value");
        }
    }

    #[test]
    fn an_empty_surface_reads_as_an_empty_snapshot_rather_than_a_failure() {
        // No fields is a legitimate answer, not an error: a surface without
        // them is an ordinary surface.
        let snapshot =
            UiFieldSnapshot::from_states(&UiFieldStates::new()).expect("the snapshot builds");
        assert!(snapshot.fields().is_empty());
    }

    #[test]
    fn a_surface_with_more_fields_than_the_bound_fails_rather_than_truncating() {
        let names: Vec<String> = (0..=MAX_SNAPSHOT_FIELDS)
            .map(|index| format!("field-{index}"))
            .collect();
        let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(
            UiFieldSnapshot::from_states(&states_for(&borrowed)),
            Err(UiFieldSnapshotError::TooManyFields)
        );
    }
}
