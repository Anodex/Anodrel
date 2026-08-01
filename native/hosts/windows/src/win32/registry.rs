//! Per-window host-owned view storage for the Win32 message loop.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, OnceLock},
};

use super::{Hwnd, StartupLab, View};

static VIEWS: OnceLock<Mutex<BTreeMap<Hwnd, View>>> = OnceLock::new();

pub(super) fn insert(window: Hwnd, view: View) -> io::Result<()> {
    let mut views = lock_views()?;
    if views.insert(window, view).is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "window view was already registered",
        ));
    }
    Ok(())
}

pub(super) fn view_for(window: Hwnd) -> io::Result<Option<View>> {
    Ok(lock_views()?.get(&window).cloned())
}

/// Mutates a window's Startup Lab state in place.
///
/// Returns `Ok(None)` when the window is absent or is not a Startup Lab, which
/// is the ordinary case for pointer and timer messages arriving at a document
/// window. Interaction state lives here rather than in the drawing code so that
/// a view stays a single owned value per window handle.
pub(super) fn with_startup_lab<R>(
    window: Hwnd,
    change: impl FnOnce(&mut StartupLab) -> R,
) -> io::Result<Option<R>> {
    let mut views = lock_views()?;
    match views.get_mut(&window) {
        Some(View::StartupLab(lab)) => Ok(Some(change(lab))),
        _ => Ok(None),
    }
}

/// Removes a window and returns the number of host windows that remain.
pub(super) fn remove(window: Hwnd) -> io::Result<usize> {
    let mut views = lock_views()?;
    views.remove(&window);
    Ok(views.len())
}

fn lock_views() -> io::Result<std::sync::MutexGuard<'static, BTreeMap<Hwnd, View>>> {
    VIEWS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| io::Error::other("window registry is unavailable"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::PackageFacts;
    use crate::win32::document::Document;
    use std::time::Instant;

    /// The registry is process-global, so tests that assert on the remaining
    /// window count must not overlap with tests that register windows.
    static EXCLUSIVE: Mutex<()> = Mutex::new(());

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
            startup_millis: 12,
            working_set_bytes: 1024,
            last_frame_micros: 0,
            revealed_at: Instant::now(),
            hovered: None,
            ambient: false,
        })
    }

    #[test]
    fn keeps_each_window_view_and_final_close_count_independent() {
        let _exclusive = EXCLUSIVE.lock().expect("registry tests are serialized");
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
        let _exclusive = EXCLUSIVE.lock().expect("registry tests are serialized");
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
    fn mutating_a_document_window_reports_no_startup_lab() {
        let _exclusive = EXCLUSIVE.lock().expect("registry tests are serialized");
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
    fn mutating_an_unknown_window_is_not_an_error() {
        assert!(
            with_startup_lab(-999, |_| ())
                .expect("mutation succeeds")
                .is_none()
        );
    }
}
