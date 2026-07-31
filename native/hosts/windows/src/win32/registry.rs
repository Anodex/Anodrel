//! Per-window host-owned view storage for the Win32 message loop.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, OnceLock},
};

use super::{Hwnd, View};

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

    #[test]
    fn keeps_each_window_view_and_final_close_count_independent() {
        let primary = -101;
        let companion = -102;
        insert(primary, View::Text(vec![1, 0])).expect("primary view registers");
        insert(companion, View::Text(vec![2, 0])).expect("companion view registers");

        let View::Text(primary_text) = view_for(primary)
            .expect("primary view lookup succeeds")
            .expect("primary view is present")
        else {
            panic!("primary view is text");
        };
        assert_eq!(primary_text, vec![1, 0]);
        assert_eq!(remove(primary).expect("primary closes"), 1);
        assert!(
            view_for(primary)
                .expect("primary lookup succeeds")
                .is_none()
        );
        assert_eq!(remove(companion).expect("companion closes"), 0);
    }
}
