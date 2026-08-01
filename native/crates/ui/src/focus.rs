//! Portable focus traversal across one visible UI layout.

use crate::{ElementId, UiEvent, UiLayout, UiLayoutItem, UiLayoutKind};

/// Focus state for visible enabled actions in a UI layout.
///
/// This value does not receive keyboard input or control an operating-system
/// focus handle. A host maps its deliberate keyboard lifecycle to
/// [`move_next`](Self::move_next), [`move_previous`](Self::move_previous), and
/// [`activate`](Self::activate).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiFocus {
    focused: Option<ElementId>,
}

impl UiFocus {
    /// Builds focus state with no current target.
    #[must_use]
    pub const fn new() -> Self {
        Self { focused: None }
    }

    /// Returns the currently focused element ID, if one remains selected.
    #[must_use]
    pub fn focused(&self) -> Option<&ElementId> {
        self.focused.as_ref()
    }

    /// Clears focus and returns the former target, if one existed.
    pub fn clear(&mut self) -> Option<ElementId> {
        self.focused.take()
    }

    /// Moves focus to the next visible enabled action in source order.
    ///
    /// Traversal wraps to the first target after the final target. When the
    /// current target no longer appears in `layout`, this starts at the first
    /// target in that new layout. Returns the selected ID, or `None` after
    /// clearing focus when no visible enabled action exists.
    pub fn move_next(&mut self, layout: &UiLayout) -> Option<ElementId> {
        let first = layout.items().iter().find(|item| is_focusable(item));
        let Some(first) = first else {
            self.focused = None;
            return None;
        };
        let Some(current) = self.focused.as_ref() else {
            return self.select(first);
        };

        let next = layout
            .items()
            .iter()
            .skip_while(|item| item.id() != current)
            .skip(1)
            .find(|item| is_focusable(item))
            .unwrap_or(first);
        self.select(next)
    }

    /// Moves focus to the previous visible enabled action in source order.
    ///
    /// Traversal wraps to the final target before the first target. When the
    /// current target no longer appears in `layout`, this starts at the final
    /// target in that new layout. Returns the selected ID, or `None` after
    /// clearing focus when no visible enabled action exists.
    pub fn move_previous(&mut self, layout: &UiLayout) -> Option<ElementId> {
        let last = layout.items().iter().rev().find(|item| is_focusable(item));
        let Some(last) = last else {
            self.focused = None;
            return None;
        };
        let Some(current) = self.focused.as_ref() else {
            return self.select(last);
        };

        let mut previous = None;
        let mut found_current = false;
        for item in layout.items() {
            if item.id() == current {
                found_current = true;
                break;
            }
            if is_focusable(item) {
                previous = Some(item);
            }
        }
        let previous = if found_current {
            previous.unwrap_or(last)
        } else {
            last
        };
        self.select(previous)
    }

    /// Returns a semantic action event for the current valid focus target.
    ///
    /// A target that disappeared, became clipped, or became disabled after a
    /// relayout cannot activate. This method has no native side effect.
    #[must_use]
    pub fn activate(&self, layout: &UiLayout) -> Option<UiEvent> {
        let current = self.focused.as_ref()?;
        layout
            .items()
            .iter()
            .find(|item| is_focusable(item) && item.id() == current)
            .map(|item| UiEvent::ActionInvoked(item.id().clone()))
    }

    fn select(&mut self, item: &UiLayoutItem) -> Option<ElementId> {
        let id = item.id().clone();
        self.focused = Some(id.clone());
        Some(id)
    }
}

fn is_focusable(item: &UiLayoutItem) -> bool {
    item.kind() == UiLayoutKind::Action && item.enabled()
}

#[cfg(test)]
mod tests {
    use crate::{
        Action, Axis, ElementId, Insets, Stack, TextMeasurer, UiDocument, UiEvent, UiNode, UiRect,
        UiSize,
    };

    use super::UiFocus;

    struct FixedMeasurer;

    impl TextMeasurer for FixedMeasurer {
        fn measure(&self, _: &str, font_size: u16) -> UiSize {
            UiSize::new(20.0, f32::from(font_size))
        }
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).expect("test ID is valid")
    }

    fn action(id_value: &str, enabled: bool) -> UiNode {
        UiNode::Action(
            Action::new(id(id_value), id_value, 10, enabled).expect("test action is valid"),
        )
    }

    fn document(children: Vec<UiNode>) -> UiDocument {
        UiDocument::new(UiNode::Stack(
            Stack::new(id("root"), Axis::Vertical, Insets::zero(), 0, children)
                .expect("test stack is valid"),
        ))
        .expect("test document is valid")
    }

    fn layout(children: Vec<UiNode>) -> crate::UiLayout {
        document(children).layout(UiRect::from_size(0.0, 0.0, 200.0, 200.0), &FixedMeasurer)
    }

    #[test]
    fn traverses_enabled_actions_in_source_order_and_wraps() {
        let layout = layout(vec![
            action("first", true),
            action("disabled", false),
            action("second", true),
        ]);
        let mut focus = UiFocus::new();

        assert_eq!(focus.move_next(&layout), Some(id("first")));
        assert_eq!(focus.move_next(&layout), Some(id("second")));
        assert_eq!(focus.move_next(&layout), Some(id("first")));
        assert_eq!(focus.move_previous(&layout), Some(id("second")));
        assert_eq!(
            focus.activate(&layout),
            Some(UiEvent::ActionInvoked(id("second")))
        );
    }

    #[test]
    fn traverses_from_the_appropriate_end_after_a_stale_relayout() {
        let layout = layout(vec![action("first", true), action("second", true)]);
        let mut focus = UiFocus {
            focused: Some(id("disappeared")),
        };

        assert_eq!(focus.move_next(&layout), Some(id("first")));
        focus.focused = Some(id("disappeared"));
        assert_eq!(focus.move_previous(&layout), Some(id("second")));
    }

    #[test]
    fn does_not_activate_a_target_that_disappeared_or_became_disabled() {
        let available = layout(vec![action("first", true)]);
        let missing = layout(vec![]);
        let disabled = layout(vec![action("first", false)]);
        let mut focus = UiFocus::new();
        assert_eq!(focus.move_next(&available), Some(id("first")));

        assert_eq!(focus.activate(&missing), None);
        assert_eq!(focus.activate(&disabled), None);
        assert_eq!(focus.move_next(&missing), None);
        assert_eq!(focus.focused(), None);
    }
}
