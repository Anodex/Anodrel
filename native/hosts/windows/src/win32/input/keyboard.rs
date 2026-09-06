//! Host-local character, editing, focus, and keyboard-scroll routing.

use super::super::*;

pub(super) fn handle_character(window: Hwnd, wparam: Wparam, lparam: Lparam) -> Lresult {
    let rect = client_rect(window);
    // Backspace reaches a window as a control character rather than an edit
    // key, so it is separated here and routed as the edit it is. Every other
    // control character is dropped: a field refuses them, and Tab and Enter
    // are already handled as navigation.
    let handled = match u32::try_from(wparam).ok().and_then(char::from_u32) {
        Some(character) if u32::from(character) == CHAR_BACKSPACE => {
            edit_focused_field(window, rect, ui_lab::FieldEdit::Backspace)
        }
        Some(character) if !character.is_control() => type_character(window, rect, character),
        _ => None,
    };
    let Some(changed) = handled else {
        // SAFETY: a character this view does not consume is forwarded unchanged
        // to the documented default Win32 procedure.
        return unsafe { DefWindowProcW(window, WM_CHAR, wparam, lparam) };
    };
    if changed {
        invalidate(window);
    }
    0
}

pub(super) fn handle_edit_key(window: Hwnd, wparam: Wparam, lparam: Lparam) -> Lresult {
    let rect = client_rect(window);
    let edit = match wparam {
        VK_LEFT => ui_lab::FieldEdit::Left,
        VK_RIGHT => ui_lab::FieldEdit::Right,
        VK_HOME => ui_lab::FieldEdit::Home,
        VK_END => ui_lab::FieldEdit::End,
        _ => ui_lab::FieldEdit::Delete,
    };
    let Some(changed) = edit_focused_field(window, rect, edit) else {
        // SAFETY: with no field focused these keys keep their default meaning
        // for the window.
        return unsafe { DefWindowProcW(window, WM_KEYDOWN, wparam, lparam) };
    };
    if changed {
        invalidate(window);
    }
    0
}

pub(super) fn handle_key_down(window: Hwnd, wparam: Wparam, lparam: Lparam) -> Lresult {
    // The menu route is deliberately before local text and focus handling, but
    // accepts only a first ordinary key-down. The three state queries describe
    // this key message's input queue state; none reaches an application or
    // protocol response.
    let shift_down = unsafe { GetKeyState(VK_SHIFT) } < 0;
    if lparam & KEY_WAS_DOWN == 0 {
        let control_down = unsafe { GetKeyState(VK_CONTROL) } < 0;
        let alt_down = unsafe { GetKeyState(VK_MENU) } < 0;
        let handled =
            registry::offer_menu_shortcut(window, wparam, control_down, shift_down, alt_down)
                .ok()
                .flatten()
                .unwrap_or(false);
        if handled {
            return 0;
        }
    }
    if !matches!(wparam, VK_TAB | VK_RETURN | VK_PRIOR | VK_NEXT) {
        // SAFETY: an unsupported key is forwarded unchanged to the documented
        // default Win32 procedure.
        return unsafe { DefWindowProcW(window, WM_KEYDOWN, wparam, lparam) };
    }
    let rect = client_rect(window);
    if matches!(wparam, VK_PRIOR | VK_NEXT) {
        let changed = registry::with_ui_lab(window, |lab| {
            lab.scroll_page(rect.width() as f32, rect.height() as f32, wparam == VK_NEXT)
        })
        .ok()
        .flatten()
        .or_else(|| {
            registry::with_ui_session(window, |session| {
                session.scroll_page(rect.width() as f32, rect.height() as f32, wparam == VK_NEXT)
            })
            .ok()
            .flatten()
        });
        let Some(changed) = changed else {
            return unsafe { DefWindowProcW(window, WM_KEYDOWN, wparam, lparam) };
        };
        if changed {
            invalidate(window);
        }
        return 0;
    }
    let changed = registry::with_ui_lab(window, |lab| match wparam {
        VK_TAB if shift_down => lab.focus_previous(rect.width() as f32, rect.height() as f32),
        VK_TAB => lab.focus_next(rect.width() as f32, rect.height() as f32),
        VK_RETURN => lab.activate_focused(rect.width() as f32, rect.height() as f32),
        _ => false,
    })
    .ok()
    .flatten()
    .or_else(|| {
        registry::with_ui_session(window, |session| match wparam {
            VK_TAB if shift_down => {
                session.focus_previous(rect.width() as f32, rect.height() as f32)
            }
            VK_TAB => session.focus_next(rect.width() as f32, rect.height() as f32),
            VK_RETURN => session.activate_focused(rect.width() as f32, rect.height() as f32),
            _ => false,
        })
        .ok()
        .flatten()
    });
    let Some(changed) = changed else {
        // Startup Lab and document views retain native default keyboard behavior
        // until their own input contracts exist.
        return unsafe { DefWindowProcW(window, WM_KEYDOWN, wparam, lparam) };
    };
    if changed {
        invalidate(window);
        if wparam == VK_TAB {
            raise_accessibility_focus_changed(window);
        }
    }
    0
}
