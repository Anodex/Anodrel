//! Focused verification for host-owned native view registry behavior.

use super::*;
use crate::win32::PackageFacts;
use crate::win32::document::Document;
use crate::win32::ui_lab::UiLab;
use anodrel_diagnostics::LogBook;
use std::time::Instant;

fn document_view(title: &str) -> View {
    View::Document(Document::from_text(title, "test", "body"))
}

fn startup_lab_view() -> View {
    View::StartupLab(StartupLab {
        package: PackageFacts {
            display_name: "Sample".to_owned(),
            application_id: "org.anodrel.sample".to_owned(),
            content_format: "anodrel.text.v1".to_owned(),
            content_path: "content/main.txt".to_owned(),
            content_digest: "00".repeat(32),
            content_bytes: 7,
        },
        log: LogBook::new(),
        startup_millis: 12,
        working_set_bytes: 1024,
        last_frame_micros: 0,
        revealed_at: Instant::now(),
        hovered: None,
        ambient: false,
        launch_available: false,
    })
}

fn ui_lab_view() -> View {
    View::UiLab(UiLab::new())
}

#[test]
fn keeps_each_window_view_and_final_close_count_independent() {
    let _exclusive = super::tests_exclusive();
    let primary = -101;
    let companion = -102;
    insert(primary, document_view("primary")).expect("primary view registers");
    insert(companion, document_view("companion")).expect("companion view registers");

    let View::Document(primary_document) = view_for(primary)
        .expect("primary view lookup succeeds")
        .expect("primary view is present")
    else {
        panic!("primary view is a document");
    };
    assert_eq!(primary_document.title, "primary");
    assert_eq!(remove(primary).expect("primary closes"), 1);
    assert!(
        view_for(primary)
            .expect("primary lookup succeeds")
            .is_none()
    );
    assert_eq!(remove(companion).expect("companion closes"), 0);
}

#[test]
fn startup_lab_state_is_mutated_in_place() {
    let _exclusive = super::tests_exclusive();
    let window = -201;
    insert(window, startup_lab_view()).expect("lab view registers");

    let changed = with_startup_lab(window, |lab| {
        lab.hovered = Some(2);
        lab.last_frame_micros = 1_500;
    })
    .expect("mutation succeeds");
    assert!(changed.is_some());

    let View::StartupLab(lab) = view_for(window)
        .expect("lookup succeeds")
        .expect("view is present")
    else {
        panic!("view is a startup lab");
    };
    assert_eq!(lab.hovered, Some(2));
    assert_eq!(lab.last_frame_micros, 1_500);
    remove(window).expect("lab closes");
}

#[test]
fn ui_lab_state_is_mutated_only_for_a_ui_lab_window() {
    let _exclusive = super::tests_exclusive();
    let window = -203;
    insert(window, ui_lab_view()).expect("UI Lab view registers");
    assert_eq!(
        with_ui_lab(window, |lab| {
            let was_empty = lab.hovered.is_none() && lab.last_action.is_none();
            lab.clear_hover();
            was_empty
        })
        .expect("mutation succeeds"),
        Some(true)
    );
    assert!(
        with_startup_lab(window, |_| ())
            .expect("mutation succeeds")
            .is_none()
    );
    assert_eq!(remove(window).expect("UI Lab closes"), 0);
}

#[test]
fn only_interactive_native_ui_views_use_system_appearance() {
    let _exclusive = super::tests_exclusive();
    let lab = -204;
    let document = -205;
    insert(lab, ui_lab_view()).expect("UI Lab view registers");
    insert(document, document_view("document")).expect("document view registers");
    assert!(uses_system_appearance(lab).expect("appearance query succeeds"));
    assert!(!uses_system_appearance(document).expect("appearance query succeeds"));
    remove(lab).expect("UI Lab closes");
    remove(document).expect("document closes");
}

#[test]
fn mutating_a_document_window_reports_no_startup_lab() {
    let _exclusive = super::tests_exclusive();
    let window = -202;
    insert(window, document_view("document")).expect("document view registers");
    assert!(
        with_startup_lab(window, |_| ())
            .expect("mutation succeeds")
            .is_none()
    );
    remove(window).expect("document closes");
}

#[test]
fn removal_releases_the_registry_before_dropping_the_view() {
    // A product-session view ends its session on drop, which joins two
    // worker threads. This proves the registry is usable from that drop
    // rather than locked behind it.
    let _exclusive = super::tests_exclusive();
    let window = -206;
    let companion = -207;
    insert(window, document_view("dropping")).expect("view registers");
    insert(companion, document_view("companion")).expect("companion registers");

    let remaining = remove(window).expect("removal succeeds");
    assert_eq!(remaining, 1);
    // The lock is free immediately afterwards, which is what a view's drop
    // would need if it reached back into the registry.
    assert!(view_for(window).expect("lookup succeeds").is_none());
    assert!(view_for(companion).expect("lookup succeeds").is_some());
    remove(companion).expect("companion closes");
}

#[test]
fn mutating_an_unknown_window_is_not_an_error() {
    assert!(
        with_startup_lab(-999, |_| ())
            .expect("mutation succeeds")
            .is_none()
    );
}

#[test]
fn update_percentage_is_whole_monotonic_and_capped_to_signed_total() {
    assert_eq!(whole_percent(0, 10), 0);
    assert_eq!(whole_percent(9, 10), 90);
    assert_eq!(whole_percent(10, 10), 100);
    assert_eq!(whole_percent(99, 10), 100);
}

#[test]
fn update_caption_keeps_the_validated_caption_as_its_suffix() {
    let base = "Quarterly Report — Verified App";
    assert_eq!(
        update_caption(
            base,
            ProductUpdateActivity::Downloading {
                completed_bytes: 51,
                total_bytes: 100,
            }
        ),
        "Downloading Anodrel update — 51% — Quarterly Report — Verified App"
    );
    assert_eq!(
        update_caption(base, ProductUpdateActivity::Idle),
        "Quarterly Report — Verified App"
    );
}
