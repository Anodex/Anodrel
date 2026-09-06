//! Keyboard, pointer, scroll, cursor, and capture routing.
//!
//! This module accepts only messages delivered to an already registered host
//! window. Its routing stays inside host-owned UI state and never accepts
//! native input targets or commands from the application protocol.

mod keyboard;
mod pointer;

use super::*;

/// Extracts the signed client coordinates packed into an `LPARAM`.
pub(super) fn mouse_position(lparam: Lparam) -> (i32, i32) {
    let raw = lparam as u32;
    ((raw & 0xFFFF) as i16 as i32, (raw >> 16) as i16 as i32)
}

pub(super) fn wheel_delta(wparam: Wparam) -> i16 {
    ((wparam >> 16) as u16) as i16
}

pub(super) fn handle_input_message(
    window: Hwnd,
    message: Uint,
    wparam: Wparam,
    lparam: Lparam,
) -> Option<Lresult> {
    match message {
        WM_CHAR => Some(keyboard::handle_character(window, wparam, lparam)),
        WM_KEYDOWN if matches!(wparam, VK_LEFT | VK_RIGHT | VK_HOME | VK_END | VK_DELETE) => {
            Some(keyboard::handle_edit_key(window, wparam, lparam))
        }
        WM_KEYDOWN => Some(keyboard::handle_key_down(window, wparam, lparam)),
        WM_MOUSEWHEEL => Some(pointer::handle_wheel(window, wparam)),
        WM_MOUSEMOVE => Some(pointer::handle_mouse_move(window, lparam)),
        WM_MOUSELEAVE => Some(pointer::handle_mouse_leave(window)),
        WM_SETCURSOR if (lparam as u32 & 0xFFFF) as isize == HTCLIENT => {
            Some(pointer::handle_set_cursor(window))
        }
        WM_LBUTTONDOWN => Some(pointer::handle_left_button_down(window, lparam)),
        WM_LBUTTONUP => Some(pointer::handle_left_button_up(window, lparam)),
        WM_CAPTURECHANGED => Some(pointer::handle_capture_changed(window)),
        _ => None,
    }
}
