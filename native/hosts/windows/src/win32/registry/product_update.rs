//! Verified-product update state kept outside the clonable window view.

use std::{
    collections::BTreeMap,
    io,
    sync::{Mutex, MutexGuard, OnceLock},
};

use super::{Hwnd, View};
use anodrel_windows_product_update::{
    ProductUpdateActivity, ProductUpdateController, ProductUpdatePoll, ProductUpdateStartError,
    UpdateConsent,
};
use anodrel_windows_taskbar_progress::TaskbarProgress;

static PRODUCT_UPDATES: OnceLock<Mutex<BTreeMap<Hwnd, ProductUpdateEntry>>> = OnceLock::new();

/// One non-cloneable update controller plus the small UI-thread-only display
/// state for the same verified product window.
pub(super) struct ProductUpdateEntry {
    controller: ProductUpdateController,
    presentation: Option<ProductUpdatePresentation>,
}

/// The fixed product-caption and optional taskbar state for one update action.
///
/// It contains no application identity, URL, filesystem path, candidate
/// version, byte count, or error. The only displayed transfer fact is a
/// whole percentage derived by the controller from signed bytes.
struct ProductUpdatePresentation {
    base_caption: String,
    displayed_caption: String,
    activity: ProductUpdateActivity,
    taskbar_ready: bool,
    displayed_taskbar: Option<TaskbarProgress>,
}

/// One UI-thread native presentation change prepared without calling Windows.
pub(in crate::win32) struct ProductUpdatePresentationChange {
    pub(in crate::win32) caption: Option<String>,
    pub(in crate::win32) taskbar: Option<TaskbarProgress>,
}

impl ProductUpdatePresentation {
    fn new(base_caption: &str) -> Self {
        Self {
            base_caption: base_caption.to_owned(),
            displayed_caption: base_caption.to_owned(),
            activity: ProductUpdateActivity::Idle,
            taskbar_ready: false,
            displayed_taskbar: None,
        }
    }

    fn set_base_caption(&mut self, base_caption: String) -> String {
        self.base_caption = base_caption;
        let next = update_caption(&self.base_caption, self.activity);
        self.displayed_caption = next.clone();
        next
    }

    fn update(&mut self, activity: ProductUpdateActivity) -> ProductUpdatePresentationChange {
        self.activity = activity;
        let next_caption = update_caption(&self.base_caption, activity);
        let caption = (next_caption != self.displayed_caption).then(|| next_caption.clone());
        self.displayed_caption = next_caption;
        let taskbar = self.next_taskbar(activity);
        ProductUpdatePresentationChange { caption, taskbar }
    }

    fn taskbar_button_created(
        &mut self,
        activity: ProductUpdateActivity,
    ) -> ProductUpdatePresentationChange {
        self.taskbar_ready = true;
        self.displayed_taskbar = None;
        self.update(activity)
    }

    fn taskbar_restarted(&mut self) {
        self.taskbar_ready = false;
        self.displayed_taskbar = None;
    }

    fn clear_taskbar(&mut self) -> Option<TaskbarProgress> {
        let shown = self.taskbar_ready && self.displayed_taskbar.is_some();
        self.taskbar_ready = false;
        self.displayed_taskbar = None;
        shown.then_some(TaskbarProgress::Clear)
    }

    fn next_taskbar(&mut self, activity: ProductUpdateActivity) -> Option<TaskbarProgress> {
        if !self.taskbar_ready {
            return None;
        }
        let next = taskbar_progress(activity);
        if next == self.displayed_taskbar {
            return None;
        }
        if next.is_none() && self.displayed_taskbar.is_none() {
            return None;
        }
        self.displayed_taskbar = next;
        Some(next.unwrap_or(TaskbarProgress::Clear))
    }
}

pub(super) fn entry_for(view: &View) -> io::Result<Option<ProductUpdateEntry>> {
    match view {
        View::UiSession(session) => session
            .product_update_application_id()
            .map(ProductUpdateController::new)
            .transpose()
            .map(|controller| {
                controller.map(|controller| ProductUpdateEntry {
                    controller,
                    presentation: None,
                })
            })
            .map_err(start_error),
        _ => Ok(None),
    }
}

/// Returns whether one exact product window has a signed-policy update action.
pub(in crate::win32) fn has_product_update_action(window: Hwnd) -> io::Result<bool> {
    Ok(lock()?.contains_key(&window))
}

/// Starts one user-chosen native product update without holding a view lock.
pub(in crate::win32) fn begin_product_update(window: Hwnd) -> io::Result<Option<bool>> {
    let mut updates = lock()?;
    let Some(entry) = updates.get_mut(&window) else {
        return Ok(None);
    };
    entry.controller.begin().map(Some).map_err(start_error)
}

/// Polls one product update worker without issuing a native prompt or holding
/// the view registry lock.
pub(in crate::win32) fn poll_product_update(window: Hwnd) -> io::Result<Option<ProductUpdatePoll>> {
    let mut updates = lock()?;
    Ok(updates
        .get_mut(&window)
        .map(|entry| entry.controller.poll()))
}

/// Submits the direct UI-thread consent decision to its same-window controller.
pub(in crate::win32) fn submit_product_update_consent(
    window: Hwnd,
    consent: UpdateConsent,
) -> io::Result<Option<ProductUpdatePoll>> {
    let mut updates = lock()?;
    updates
        .get_mut(&window)
        .map(|entry| entry.controller.submit_consent(consent))
        .transpose()
        .map_err(start_error)
}

/// Returns whether a window's native product update still owns a worker.
pub(in crate::win32) fn product_update_is_active(window: Hwnd) -> io::Result<Option<bool>> {
    let updates = lock()?;
    Ok(updates
        .get(&window)
        .map(|entry| entry.controller.is_active()))
}

/// Enables fixed update progress presentation once the native system-menu
/// action exists for this verified product window.
pub(in crate::win32) fn prepare_product_update_presentation(
    window: Hwnd,
    base_caption: &str,
) -> io::Result<bool> {
    let mut updates = lock()?;
    let Some(entry) = updates.get_mut(&window) else {
        return Ok(false);
    };
    entry.presentation = Some(ProductUpdatePresentation::new(base_caption));
    Ok(true)
}

/// Replaces a product update's base caption and returns its host-composed form.
pub(in crate::win32) fn compose_product_update_caption(
    window: Hwnd,
    base_caption: String,
) -> io::Result<String> {
    let mut updates = lock()?;
    let Some(entry) = updates.get_mut(&window) else {
        return Ok(base_caption);
    };
    Ok(entry
        .presentation
        .as_mut()
        .map_or(base_caption.clone(), |presentation| {
            presentation.set_base_caption(base_caption)
        }))
}

/// Collects a current host-only progress presentation without calling Windows.
pub(in crate::win32) fn refresh_product_update_presentation(
    window: Hwnd,
) -> io::Result<Option<ProductUpdatePresentationChange>> {
    let mut updates = lock()?;
    let Some(entry) = updates.get_mut(&window) else {
        return Ok(None);
    };
    Ok(entry
        .presentation
        .as_mut()
        .map(|presentation| presentation.update(entry.controller.activity())))
}

/// Marks this window taskbar-ready only after Windows sent its required signal.
pub(in crate::win32) fn product_update_taskbar_button_created(
    window: Hwnd,
) -> io::Result<Option<ProductUpdatePresentationChange>> {
    let mut updates = lock()?;
    let Some(entry) = updates.get_mut(&window) else {
        return Ok(None);
    };
    Ok(entry
        .presentation
        .as_mut()
        .map(|presentation| presentation.taskbar_button_created(entry.controller.activity())))
}

/// Drops taskbar readiness after Windows announces that its taskbar restarted.
pub(in crate::win32) fn product_update_taskbar_restarted(window: Hwnd) -> io::Result<()> {
    let mut updates = lock()?;
    if let Some(presentation) = updates
        .get_mut(&window)
        .and_then(|entry| entry.presentation.as_mut())
    {
        presentation.taskbar_restarted();
    }
    Ok(())
}

/// Clears a currently visible taskbar state before a product window ends.
pub(in crate::win32) fn clear_product_update_taskbar(
    window: Hwnd,
) -> io::Result<Option<TaskbarProgress>> {
    let mut updates = lock()?;
    Ok(updates
        .get_mut(&window)
        .and_then(|entry| entry.presentation.as_mut())
        .and_then(ProductUpdatePresentation::clear_taskbar))
}

pub(super) fn lock() -> io::Result<MutexGuard<'static, BTreeMap<Hwnd, ProductUpdateEntry>>> {
    PRODUCT_UPDATES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| io::Error::other("product update registry is unavailable"))
}

fn start_error(error: ProductUpdateStartError) -> io::Error {
    let _ = error;
    io::Error::other("product update action is unavailable")
}

pub(super) fn update_caption(base_caption: &str, activity: ProductUpdateActivity) -> String {
    match activity {
        ProductUpdateActivity::Idle => base_caption.to_owned(),
        ProductUpdateActivity::Discovering => {
            format!("Checking for Anodrel updates — {base_caption}")
        }
        ProductUpdateActivity::Downloading {
            completed_bytes,
            total_bytes,
        } => format!(
            "Downloading Anodrel update — {}% — {base_caption}",
            whole_percent(completed_bytes, total_bytes)
        ),
        ProductUpdateActivity::Installing => format!("Installing Anodrel update — {base_caption}"),
    }
}

pub(super) const fn whole_percent(completed_bytes: u64, total_bytes: u64) -> u8 {
    if total_bytes == 0 {
        return 0;
    }
    let completed = if completed_bytes > total_bytes {
        total_bytes
    } else {
        completed_bytes
    };
    let whole = (completed / total_bytes) * 100;
    let remainder = ((completed % total_bytes) * 100) / total_bytes;
    (whole + remainder) as u8
}

const fn taskbar_progress(activity: ProductUpdateActivity) -> Option<TaskbarProgress> {
    match activity {
        ProductUpdateActivity::Idle => None,
        ProductUpdateActivity::Discovering | ProductUpdateActivity::Installing => {
            Some(TaskbarProgress::Activity)
        }
        ProductUpdateActivity::Downloading {
            completed_bytes,
            total_bytes,
        } => Some(TaskbarProgress::Determinate {
            completed: completed_bytes,
            total: total_bytes,
        }),
    }
}
