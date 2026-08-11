//! Host-owned text and caret for the fields on one surface.
//!
//! This is where what a person types lives. It is portable and has no
//! operating-system dependency, so the editing rules are unit-tested rather
//! than discovered by typing into a window.
//!
//! Nothing here produces an event, a callback, or a notification. A host reads
//! this state to draw a field and, separately and only when granted, to answer
//! a snapshot request. An application cannot observe an edit as it happens; see
//! `docs/UI_FIELDS.md` and Decision 0067.

use std::collections::BTreeMap;

use crate::{ElementId, Field, UiDocument, UiNode};

/// The text and caret of one field.
///
/// The caret is a byte offset that always sits on a character boundary. Every
/// operation here maintains that, so a host can slice the text for drawing
/// without checking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiFieldState {
    text: String,
    caret: usize,
}

impl UiFieldState {
    /// Builds state holding a field's starting value, caret at the end.
    ///
    /// The caret starts after the text because a field arriving with a value is
    /// usually one a person is about to add to or correct.
    #[must_use]
    pub fn from_field(field: &Field) -> Self {
        let text = field.value().to_owned();
        let caret = text.len();
        Self { text, caret }
    }

    /// Returns the current text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the caret's byte offset, always on a character boundary.
    #[must_use]
    pub const fn caret(&self) -> usize {
        self.caret
    }

    /// Returns the number of characters currently held.
    #[must_use]
    pub fn character_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Inserts one character at the caret, respecting the field's limit.
    ///
    /// Returns `false` when the character was refused, which happens when the
    /// field is full or the character is a control character. Refusing rather
    /// than filtering silently is what keeps a paste of `a\nb` from becoming
    /// `ab`: the host decides what to do about a rejected character, and the
    /// text never contains one.
    pub fn insert(&mut self, character: char, field: &Field) -> bool {
        if character.is_control() {
            return false;
        }
        if self.character_count() >= usize::from(field.max_length()) {
            return false;
        }
        self.text.insert(self.caret, character);
        self.caret += character.len_utf8();
        true
    }

    /// Removes the character before the caret.
    ///
    /// Returns `false` when the caret is already at the start.
    pub fn backspace(&mut self) -> bool {
        let Some(previous) = self.previous_boundary() else {
            return false;
        };
        self.text.replace_range(previous..self.caret, "");
        self.caret = previous;
        true
    }

    /// Removes the character after the caret.
    ///
    /// Returns `false` when the caret is already at the end.
    pub fn delete(&mut self) -> bool {
        let Some(next) = self.next_boundary() else {
            return false;
        };
        self.text.replace_range(self.caret..next, "");
        true
    }

    /// Moves the caret one character left. Returns `false` at the start.
    pub fn move_left(&mut self) -> bool {
        match self.previous_boundary() {
            Some(previous) => {
                self.caret = previous;
                true
            }
            None => false,
        }
    }

    /// Moves the caret one character right. Returns `false` at the end.
    pub fn move_right(&mut self) -> bool {
        match self.next_boundary() {
            Some(next) => {
                self.caret = next;
                true
            }
            None => false,
        }
    }

    /// Moves the caret before the first character.
    pub const fn move_home(&mut self) {
        self.caret = 0;
    }

    /// Moves the caret after the last character.
    pub const fn move_end(&mut self) {
        self.caret = self.text.len();
    }

    /// Returns the byte offset of the character boundary before the caret.
    fn previous_boundary(&self) -> Option<usize> {
        if self.caret == 0 {
            return None;
        }
        self.text[..self.caret]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    /// Returns the byte offset of the character boundary after the caret.
    fn next_boundary(&self) -> Option<usize> {
        self.text[self.caret..]
            .chars()
            .next()
            .map(|character| self.caret + character.len_utf8())
    }
}

/// Host-owned field text keyed by element ID.
///
/// Held beside a surface's layout the way scroll offsets are. It belongs to the
/// host, and no protocol operation writes to it: a document sets a field's
/// starting value, and after that only a person changes it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiFieldStates {
    states: BTreeMap<ElementId, UiFieldState>,
}

impl UiFieldStates {
    /// Creates empty field state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns one field's state, if it has been seeded.
    #[must_use]
    pub fn get(&self, id: &ElementId) -> Option<&UiFieldState> {
        self.states.get(id)
    }

    /// Returns one field's state for editing, if it has been seeded.
    pub fn get_mut(&mut self, id: &ElementId) -> Option<&mut UiFieldState> {
        self.states.get_mut(id)
    }

    /// Returns how many fields currently hold state.
    #[must_use]
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Returns whether any field holds state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Replaces all state from the fields of a newly applied document.
    ///
    /// Every field is seeded from the document and every field absent from it
    /// is dropped. A document is a whole snapshot rather than a patch, so
    /// republishing one **discards what a person had typed** — the alternative
    /// is two disagreeing sources of truth with no rule for which wins. Decision
    /// 0067 states this, because an application that republishes on a timer
    /// would erase input.
    pub fn reseed(&mut self, document: &UiDocument) {
        self.states.clear();
        seed_node(document.root(), &mut self.states);
    }
}

fn seed_node(node: &UiNode, states: &mut BTreeMap<ElementId, UiFieldState>) {
    match node {
        UiNode::Field(field) => {
            states.insert(field.id().clone(), UiFieldState::from_field(field));
        }
        UiNode::Stack(stack) => {
            for child in stack.children() {
                seed_node(child, states);
            }
        }
        UiNode::Scroll(scroll) => seed_node(scroll.child(), states),
        UiNode::Text(_) | UiNode::Action(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{UiFieldState, UiFieldStates};
    use crate::{Axis, ElementId, Field, Insets, Scroll, Stack, UiDocument, UiNode};

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn field(value: &str, max_length: u16) -> Field {
        Field::new(id("name"), "Name", value, max_length, 14, true).expect("test field is valid")
    }

    fn typed(text: &str, max_length: u16) -> UiFieldState {
        let field = field("", max_length);
        let mut state = UiFieldState::from_field(&field);
        for character in text.chars() {
            assert!(state.insert(character, &field), "{character:?} was refused");
        }
        state
    }

    #[test]
    fn a_seeded_field_starts_with_its_value_and_the_caret_after_it() {
        let state = UiFieldState::from_field(&field("Ada", 64));
        assert_eq!(state.text(), "Ada");
        assert_eq!(state.caret(), 3);
    }

    #[test]
    fn typing_inserts_at_the_caret_rather_than_appending() {
        let mut state = typed("Ada", 64);
        state.move_home();
        assert!(state.insert('!', &field("", 64)));
        assert_eq!(state.text(), "!Ada");
        assert_eq!(state.caret(), 1);
    }

    #[test]
    fn a_full_field_refuses_more_characters() {
        let field = field("", 3);
        let mut state = typed("abc", 3);
        assert!(!state.insert('d', &field));
        assert_eq!(state.text(), "abc");
    }

    #[test]
    fn the_limit_counts_characters_rather_than_bytes() {
        // Four emoji are four characters and sixteen bytes. The limit is what a
        // person may type, so it counts what they type.
        let field = field("", 4);
        let mut state = UiFieldState::from_field(&field);
        for _ in 0..4 {
            assert!(state.insert('\u{1F680}', &field));
        }
        assert_eq!(state.character_count(), 4);
        assert_eq!(state.text().len(), 16);
        assert!(!state.insert('\u{1F680}', &field));
    }

    #[test]
    fn a_control_character_is_refused_rather_than_filtered() {
        // Filtering would turn a pasted "a\nb" into "ab", quietly joining two
        // lines a person meant to keep apart. Refusing leaves the decision with
        // the host and keeps a control character out of the text entirely.
        let field = field("", 64);
        let mut state = UiFieldState::from_field(&field);
        for refused in ['\n', '\r', '\t', '\u{0}', '\u{1B}'] {
            assert!(!state.insert(refused, &field), "{refused:?} was accepted");
        }
        assert_eq!(state.text(), "");
    }

    #[test]
    fn editing_moves_across_whole_characters_not_bytes() {
        // A caret that landed inside a multi-byte character would panic the
        // moment the host sliced the text to draw it.
        let mut state = typed("a\u{1F680}b", 64);
        assert_eq!(state.text().len(), 6);

        state.move_home();
        assert_eq!(state.caret(), 0);
        assert!(state.move_right());
        assert_eq!(state.caret(), 1);
        assert!(state.move_right());
        assert_eq!(state.caret(), 5, "the caret skipped the whole emoji");
        assert!(state.move_left());
        assert_eq!(state.caret(), 1);

        state.move_end();
        assert!(state.backspace());
        assert_eq!(state.text(), "a\u{1F680}");
        assert!(state.backspace());
        assert_eq!(state.text(), "a");
    }

    #[test]
    fn deleting_removes_the_character_after_the_caret() {
        let mut state = typed("abc", 64);
        state.move_home();
        assert!(state.delete());
        assert_eq!(state.text(), "bc");
        assert_eq!(state.caret(), 0);
    }

    #[test]
    fn editing_at_an_edge_reports_that_nothing_happened() {
        let mut state = typed("a", 64);
        state.move_home();
        assert!(!state.backspace());
        assert!(!state.move_left());
        state.move_end();
        assert!(!state.delete());
        assert!(!state.move_right());
        assert_eq!(state.text(), "a");
    }

    fn document_with(values: &[(&str, &str)]) -> UiDocument {
        let children = values
            .iter()
            .map(|(field_id, value)| {
                UiNode::Field(
                    Field::new(id(field_id), "Label", *value, 64, 14, true)
                        .expect("test field is valid"),
                )
            })
            .collect();
        UiDocument::new(UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        ))
        .expect("test document is valid")
    }

    #[test]
    fn reseeding_takes_every_field_from_the_document_and_drops_the_rest() {
        let mut states = UiFieldStates::new();
        assert!(states.is_empty());

        states.reseed(&document_with(&[("first", "one"), ("second", "two")]));
        assert_eq!(states.len(), 2);
        assert_eq!(
            states.get(&id("first")).map(UiFieldState::text),
            Some("one")
        );

        // A document is a whole snapshot, so a field it no longer names loses
        // its state entirely rather than lingering invisibly.
        states.reseed(&document_with(&[("second", "two")]));
        assert_eq!(states.len(), 1);
        assert!(states.get(&id("first")).is_none());
    }

    #[test]
    fn republishing_a_document_discards_what_a_person_typed() {
        // The consequence Decision 0067 names out loud: an application that
        // republishes on a timer erases input. Asserted so nobody has to
        // discover it by losing a form.
        let mut states = UiFieldStates::new();
        states.reseed(&document_with(&[("name", "")]));
        let field = field("", 64);
        let state = states.get_mut(&id("name")).expect("field was seeded");
        for character in "typed by a person".chars() {
            assert!(state.insert(character, &field));
        }
        assert_eq!(
            states.get(&id("name")).map(UiFieldState::text),
            Some("typed by a person")
        );

        states.reseed(&document_with(&[("name", "")]));
        assert_eq!(states.get(&id("name")).map(UiFieldState::text), Some(""));
    }

    #[test]
    fn reseeding_reaches_fields_nested_below_stacks_and_scrolls() {
        let inner = UiNode::Field(
            Field::new(id("deep"), "Label", "value", 64, 14, true).expect("test field is valid"),
        );
        let document = UiDocument::new(UiNode::Stack(
            Stack::new(
                id("root"),
                Axis::Vertical,
                Insets::zero(),
                0,
                vec![UiNode::Scroll(Scroll::new(id("scroll"), inner))],
            )
            .expect("test stack is valid"),
        ))
        .expect("test document is valid");

        let mut states = UiFieldStates::new();
        states.reseed(&document);
        assert_eq!(
            states.get(&id("deep")).map(UiFieldState::text),
            Some("value")
        );
    }
}
