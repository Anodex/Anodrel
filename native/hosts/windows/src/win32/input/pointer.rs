//! Host-local pointer, cursor, scrollbar, and capture routing.

use std::mem;

use super::super::*;
use super::{mouse_position, wheel_delta};

pub(super) fn handle_wheel(window: Hwnd, wparam: Wparam) -> Lresult {
    let delta = wheel_delta(wparam);
    if delta == 0 {
        return 0;
    }
    let rect = client_rect(window);
    let changed = registry::with_ui_lab(window, |lab| {
        lab.scroll_wheel_delta(rect.width() as f32, rect.height() as f32, i32::from(delta))
    })
    .ok()
    .flatten()
    .or_else(|| {
        registry::with_ui_session(window, |session| {
            session.scroll_wheel_delta(rect.width() as f32, rect.height() as f32, i32::from(delta))
        })
        .ok()
        .flatten()
    });
    if changed.unwrap_or(false) {
        invalidate(window);
    }
    0
}

pub(super) fn handle_mouse_move(window: Hwnd, lparam: Lparam) -> Lresult {
    let (x, y) = mouse_position(lparam);
    let rect = client_rect(window);
    let scrollbar_changed = registry::with_ui_lab(window, |lab| {
        lab.drag_scrollbar(
            rect.width() as f32,
            rect.height() as f32,
            point(x as f32, y as f32),
        )
    })
    .ok()
    .flatten()
    .unwrap_or_else(|| {
        registry::with_ui_session(window, |session| {
            session.drag_scrollbar(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
        })
        .ok()
        .flatten()
        .unwrap_or(false)
    });
    if scrollbar_changed {
        invalidate(window);
    }
    let changed = registry::with_ui_lab(window, |lab| {
        lab.update_hover(
            rect.width() as f32,
            rect.height() as f32,
            point(x as f32, y as f32),
        )
    })
    .ok()
    .flatten()
    .unwrap_or_else(|| {
        registry::with_ui_session(window, |session| {
            session.update_hover(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
        })
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            let hit = startup_lab::action_at(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            );
            registry::with_startup_lab(window, |lab| {
                // Hover follows the same availability value as drawing and
                // clicking, so a planned tile never highlights.
                let hovered = hit
                    .filter(|index| startup_lab::tile_is_live(&startup_lab::ACTIONS[*index], lab));
                let changed = lab.hovered != hovered;
                lab.hovered = hovered;
                changed
            })
            .ok()
            .flatten()
            .unwrap_or(false)
        })
    });
    if changed {
        invalidate(window);
    }
    let mut track = TrackMouseEventStruct {
        cbSize: mem::size_of::<TrackMouseEventStruct>() as Dword,
        dwFlags: TME_LEAVE,
        hwndTrack: window,
        dwHoverTime: 0,
    };
    // SAFETY: `track` is writable stack storage whose declared size matches the
    // struct, and the window belongs to this process.
    unsafe {
        TrackMouseEvent(&mut track);
    }
    0
}

pub(super) fn handle_mouse_leave(window: Hwnd) -> Lresult {
    let changed = registry::with_ui_lab(window, ui_lab::UiLab::clear_hover)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            registry::with_ui_session(window, |session| session.clear_hover())
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    registry::with_startup_lab(window, |lab| {
                        let changed = lab.hovered.is_some();
                        lab.hovered = None;
                        changed
                    })
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                })
        });
    if changed {
        invalidate(window);
    }
    0
}

pub(super) fn handle_set_cursor(window: Hwnd) -> Lresult {
    let hovered = registry::with_ui_lab(window, |lab| lab.hovered.is_some())
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            registry::with_ui_session(window, |session| session.is_hovered())
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    registry::with_startup_lab(window, |lab| lab.hovered)
                        .ok()
                        .flatten()
                        .flatten()
                        .is_some()
                })
        });
    let cursor_id = if hovered { IDC_HAND } else { IDC_ARROW };
    // SAFETY: both identifiers are documented integer resources, and
    // LoadCursorW returns a shared cursor that must not be destroyed.
    unsafe {
        SetCursor(LoadCursorW(0, cursor_id as *const u16));
    }
    1
}

pub(super) fn handle_left_button_down(window: Hwnd, lparam: Lparam) -> Lresult {
    let (x, y) = mouse_position(lparam);
    let rect = client_rect(window);
    let began_drag = registry::with_ui_lab(window, |lab| {
        lab.begin_scrollbar_drag(
            rect.width() as f32,
            rect.height() as f32,
            point(x as f32, y as f32),
        )
    })
    .ok()
    .flatten()
    .unwrap_or_else(|| {
        registry::with_ui_session(window, |session| {
            session.begin_scrollbar_drag(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
        })
        .ok()
        .flatten()
        .unwrap_or(false)
    });
    if began_drag {
        // SAFETY: this message belongs to the registered native window that
        // owns the local thumb-drag state. Capture lasts only until that state
        // is cleared below.
        unsafe {
            SetCapture(window);
        }
        invalidate(window);
    }
    0
}

pub(super) fn handle_left_button_up(window: Hwnd, lparam: Lparam) -> Lresult {
    let (x, y) = mouse_position(lparam);
    let rect = client_rect(window);
    let (width, height) = (rect.width() as f32, rect.height() as f32);
    let at = point(x as f32, y as f32);
    let ended_drag = registry::with_ui_lab(window, ui_lab::UiLab::end_scrollbar_drag)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            registry::with_ui_session(window, |session| session.end_scrollbar_drag())
                .ok()
                .flatten()
                .unwrap_or(false)
        });
    if ended_drag {
        // SAFETY: only this window starts the matching capture above, and the
        // call merely releases the current thread's capture.
        unsafe {
            ReleaseCapture();
        }
        return 0;
    }
    let scrollbar_consumed =
        registry::with_ui_lab(window, |lab| lab.page_scrollbar_at(width, height, at))
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                registry::with_ui_session(window, |session| {
                    session.page_scrollbar_at(width, height, at)
                })
                .ok()
                .flatten()
                .unwrap_or(false)
            });
    if scrollbar_consumed {
        invalidate(window);
        return 0;
    }
    // Focus first, then invoke. A click on a field only moves focus, and a
    // click on an action does both — pressing a control is also how a person
    // expects to focus it. Treating these as alternatives is what left a field
    // unreachable by pointer.
    if let Some((changed, focus_changed)) = registry::with_ui_lab(window, |lab| {
        let focused = lab.focus_at(width, height, at);
        (lab.invoke(width, height, at) || focused, focused)
    })
    .ok()
    .flatten()
    {
        if changed {
            invalidate(window);
        }
        if focus_changed {
            raise_accessibility_focus_changed(window);
        }
        return 0;
    }
    if let Some((changed, focus_changed)) = registry::with_ui_session(window, |session| {
        let focused = session.focus_at(width, height, at);
        (session.invoke(width, height, at) || focused, focused)
    })
    .ok()
    .flatten()
    {
        if changed {
            invalidate(window);
        }
        if focus_changed {
            raise_accessibility_focus_changed(window);
        }
        return 0;
    }
    let hit = startup_lab::action_at(
        rect.width() as f32,
        rect.height() as f32,
        point(x as f32, y as f32),
    )
    .map(|index| &startup_lab::ACTIONS[index]);
    if let Some(action) = hit
        && let Ok(Some(View::StartupLab(lab))) = registry::view_for(window)
        // Hit-testing and drawing read the same availability value, so a tile
        // drawn as planned cannot be activated by a click.
        && startup_lab::tile_is_live(action, &lab)
    {
        if action.kind == startup_lab::ActionKind::LaunchDevelopmentFixture {
            begin_product_session(window);
        } else if let Some((title, document)) = action_document(action.kind, &lab) {
            // A failure to open a diagnostic window is not fatal to the surface
            // that launched it.
            let _ = open_document_window(&title, document);
        }
    }
    0
}

pub(super) fn handle_capture_changed(window: Hwnd) -> Lresult {
    let _ = registry::with_ui_lab(window, ui_lab::UiLab::end_scrollbar_drag)
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| session.end_scrollbar_drag())
                .ok()
                .flatten()
        });
    0
}
