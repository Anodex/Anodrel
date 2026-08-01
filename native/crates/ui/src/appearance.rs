//! Portable semantic appearance roles for native UI nodes.
//!
//! These roles deliberately do not carry colours, fonts, pixels, renderer
//! handles, or operating-system values. A host renderer maps them to its own
//! visual system after layout has completed.

/// The requested treatment for a stack surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiSurfaceTone {
    /// Draw no surface behind the stack's children.
    #[default]
    Plain,
    /// Draw a visually lifted grouping surface behind the stack's children.
    Raised,
}

/// The semantic prominence of a text run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiTextTone {
    /// Primary reading text, such as a title or essential body text.
    #[default]
    Primary,
    /// Supporting reading text with lower visual prominence.
    Secondary,
    /// Text that calls attention to a meaningful state or section.
    Accent,
}

/// The semantic prominence of an action's affordance.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiActionTone {
    /// A normal action with no requested emphasis.
    #[default]
    Neutral,
    /// An action the document wants the host to visually emphasize.
    Accent,
}

#[cfg(test)]
mod tests {
    use super::{UiActionTone, UiSurfaceTone, UiTextTone};

    #[test]
    fn defaults_are_the_least_emphatic_roles() {
        assert_eq!(UiSurfaceTone::default(), UiSurfaceTone::Plain);
        assert_eq!(UiTextTone::default(), UiTextTone::Primary);
        assert_eq!(UiActionTone::default(), UiActionTone::Neutral);
    }
}
