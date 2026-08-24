//! Fixed semantic document used by the Windows UI Lab.

use super::*;

pub(super) fn document() -> UiDocument {
    let fixture =
        decode(UI_LAB_DOCUMENT_JSON).expect("compiled UI Lab document matches the v1 contract");
    let scroll_exercises = UiNode::Stack(
        Stack::new(
            ElementId::new("ui.lab.scroll.exercises").expect("fixed scroll ID is valid"),
            Axis::Vertical,
            Insets::all(18).expect("fixed scroll padding is valid"),
            10,
            (1..=9)
                .map(|index| {
                    UiNode::Action(
                        Action::new(
                            ElementId::new(format!("ui.lab.scroll.exercise-{index}"))
                                .expect("fixed scroll action ID is valid"),
                            format!("Scroll exercise {index}"),
                            15,
                            true,
                        )
                        .expect("fixed scroll action is valid"),
                    )
                })
                .collect(),
        )
        .expect("fixed scroll stack is valid")
        .with_surface_tone(UiSurfaceTone::Raised),
    );
    UiDocument::new(UiNode::Scroll(Scroll::new(
        ElementId::new("ui.lab.viewport").expect("fixed scroll viewport ID is valid"),
        UiNode::Stack(
            Stack::new(
                ElementId::new("ui.lab.scroll.content").expect("fixed scroll content ID is valid"),
                Axis::Vertical,
                Insets::zero(),
                18,
                vec![fixture.root().clone(), scroll_exercises],
            )
            .expect("fixed scroll content stack is valid"),
        ),
    )))
    .expect("fixed scroll document is valid")
}
