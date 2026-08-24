//! UI Lab retained-renderer coverage.

use super::*;

#[test]
fn preview_documents_have_no_lab_specific_status_replacement() {
    let document = decode(
        r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"External text","fontSize":16,"tone":"primary"}}"#,
    )
    .expect("preview fixture is valid");
    let preview = UiLab::preview(document);

    assert!(preview.status_target.is_none());
    assert_eq!(status_text(&preview), None);
}

#[test]
fn preview_document_renders_through_the_same_native_ui_view() {
    let document = decode(
        r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"stack","axis":"vertical","padding":{"left":40,"top":40,"right":40,"bottom":40},"gap":12,"surfaceTone":"plain","children":[{"id":"title","kind":"text","value":"External preview document","fontSize":28,"tone":"primary"},{"id":"continue","kind":"action","label":"Continue","fontSize":16,"enabled":true,"tone":"accent"}]}}"#,
    )
    .expect("preview fixture is valid");
    let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    draw(&mut canvas, &UiLab::preview(document));

    let changed = changed_pixels(&canvas);
    assert!(changed > 1_000, "preview drew too little content");
}

#[test]
fn draws_visible_content_without_a_web_surface() {
    let mut canvas = Canvas::new(BASE_WIDTH as u32, BASE_HEIGHT as u32);
    draw(&mut canvas, &UiLab::new());

    let changed = changed_pixels(&canvas);
    assert!(changed > 1_000, "UI Lab drew too little content");
}

fn changed_pixels(canvas: &Canvas) -> usize {
    (0..canvas.height())
        .flat_map(|y| (0..canvas.width()).map(move |x| (x as i32, y as i32)))
        .filter(|(x, y)| canvas.pixel(*x, *y) != palette::BACKDROP)
        .count()
}
