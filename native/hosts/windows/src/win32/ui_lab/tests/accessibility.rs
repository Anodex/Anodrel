use super::super::*;
use super::id;

#[test]
fn visual_hierarchy_comes_from_semantic_roles_not_element_names() {
    let lab = UiLab::new();
    let UiNode::Scroll(viewport) = lab.document.root() else {
        panic!("fixed UI Lab root is a scroll viewport");
    };
    let UiNode::Stack(content) = viewport.child() else {
        panic!("fixed UI Lab viewport has a content stack");
    };
    let UiNode::Stack(root) = &content.children()[0] else {
        panic!("fixed UI Lab fixture is a stack");
    };

    let eyebrow = match &root.children()[0] {
        UiNode::Text(text) => text,
        _ => panic!("fixed UI Lab eyebrow is text"),
    };
    let detail = match &root.children()[2] {
        UiNode::Text(text) => text,
        _ => panic!("fixed UI Lab detail is text"),
    };
    let UiNode::Stack(actions) = &root.children()[3] else {
        panic!("fixed UI Lab actions are a stack");
    };
    // Found by ID rather than by position: this document gains nodes over
    // time, and an index would keep silently pointing at a different one.
    let emphasized_action = actions
        .children()
        .iter()
        .find_map(|child| match child {
            UiNode::Action(action) if action.id() == &id("ui.lab.hit-test") => Some(action),
            _ => None,
        })
        .expect("fixed UI Lab emphasized action exists");

    assert_eq!(eyebrow.tone(), UiTextTone::Accent);
    assert_eq!(detail.tone(), UiTextTone::Secondary);
    assert_eq!(actions.surface_tone(), UiSurfaceTone::Raised);
    assert_eq!(emphasized_action.tone(), UiActionTone::Accent);
}
