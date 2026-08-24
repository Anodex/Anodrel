//! Keyboard, pointer, scroll, cursor, and capture routing.
//!
//! This module accepts only messages delivered to an already registered host
//! window. Its routing stays inside host-owned UI state and never accepts
//! native input targets or commands from the application protocol.

use std::mem;

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
        WM_CHAR => {
            let rect = client_rect(window);
            // Backspace reaches a window as a control character rather than an
            // edit key, so it is separated here and routed as the edit it is.
            // Every other control character is dropped: a field refuses them,
            // and Tab and Enter are already handled as navigation.
            let handled = match u32::try_from(wparam).ok().and_then(char::from_u32) {
                Some(character) if u32::from(character) == CHAR_BACKSPACE => {
                    edit_focused_field(window, rect, ui_lab::FieldEdit::Backspace)
                }
                Some(character) if !character.is_control() => {
                    type_character(window, rect, character)
                }
                _ => None,
            };
            let Some(changed) = handled else {
                // SAFETY: a character this view does not consume is forwarded
                // unchanged to the documented default Win32 procedure.
                return Some(unsafe { DefWindowProcW(window, message, wparam, lparam) });
            };
            if changed {
                invalidate(window);
            }
            Some(0)
        }
        WM_KEYDOWN if matches!(wparam, VK_LEFT | VK_RIGHT | VK_HOME | VK_END | VK_DELETE) => {
            let rect = client_rect(window);
            let edit = match wparam {
                VK_LEFT => ui_lab::FieldEdit::Left,
                VK_RIGHT => ui_lab::FieldEdit::Right,
                VK_HOME => ui_lab::FieldEdit::Home,
                VK_END => ui_lab::FieldEdit::End,
                _ => ui_lab::FieldEdit::Delete,
            };
            let Some(changed) = edit_focused_field(window, rect, edit) else {
                // SAFETY: with no field focused these keys keep their default
                // meaning for the window.
                return Some(unsafe { DefWindowProcW(window, message, wparam, lparam) });
            };
            if changed {
                invalidate(window);
            }
            Some(0)
        }
        WM_KEYDOWN => {
            // The menu route is deliberately before local text and focus
            // handling, but accepts only a first ordinary key-down. The three
            // state queries describe this key message's input queue state;
            // none reaches an application or protocol response.
            let shift_down = unsafe { GetKeyState(VK_SHIFT) } < 0;
            if lparam & KEY_WAS_DOWN == 0 {
                let control_down = unsafe { GetKeyState(VK_CONTROL) } < 0;
                let alt_down = unsafe { GetKeyState(VK_MENU) } < 0;
                let handled = registry::offer_menu_shortcut(
                    window,
                    wparam,
                    control_down,
                    shift_down,
                    alt_down,
                )
                .ok()
                .flatten()
                .unwrap_or(false);
                if handled {
                    return Some(0);
                }
            }
            if !matches!(wparam, VK_TAB | VK_RETURN | VK_PRIOR | VK_NEXT) {
                // SAFETY: an unsupported key is forwarded unchanged to the
                // documented default Win32 procedure.
                return Some(unsafe { DefWindowProcW(window, message, wparam, lparam) });
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
                        session.scroll_page(
                            rect.width() as f32,
                            rect.height() as f32,
                            wparam == VK_NEXT,
                        )
                    })
                    .ok()
                    .flatten()
                });
                let Some(changed) = changed else {
                    return Some(unsafe { DefWindowProcW(window, message, wparam, lparam) });
                };
                if changed {
                    invalidate(window);
                }
                return Some(0);
            }
            let changed = registry::with_ui_lab(window, |lab| match wparam {
                VK_TAB if shift_down => {
                    lab.focus_previous(rect.width() as f32, rect.height() as f32)
                }
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
                    VK_RETURN => {
                        session.activate_focused(rect.width() as f32, rect.height() as f32)
                    }
                    _ => false,
                })
                .ok()
                .flatten()
            });
            let Some(changed) = changed else {
                // Startup Lab and document views retain native default keyboard
                // behavior until their own input contracts exist.
                return Some(unsafe { DefWindowProcW(window, message, wparam, lparam) });
            };
            if changed {
                invalidate(window);
                if wparam == VK_TAB {
                    raise_accessibility_focus_changed(window);
                }
            }
            Some(0)
        }
        WM_MOUSEWHEEL => {
            let delta = wheel_delta(wparam);
            if delta == 0 {
                return Some(0);
            }
            let rect = client_rect(window);
            let changed = registry::with_ui_lab(window, |lab| {
                lab.scroll_wheel_delta(rect.width() as f32, rect.height() as f32, i32::from(delta))
            })
            .ok()
            .flatten()
            .or_else(|| {
                registry::with_ui_session(window, |session| {
                    session.scroll_wheel_delta(
                        rect.width() as f32,
                        rect.height() as f32,
                        i32::from(delta),
                    )
                })
                .ok()
                .flatten()
            });
            if changed.unwrap_or(false) {
                invalidate(window);
            }
            Some(0)
        }
        WM_MOUSEMOVE => {
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
                        // Hover follows the same availability value as drawing
                        // and clicking, so a planned tile never highlights.
                        let hovered = hit.filter(|index| {
                            startup_lab::tile_is_live(&startup_lab::ACTIONS[*index], lab)
                        });
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
            // SAFETY: `track` is writable stack storage whose declared size
            // matches the struct, and the window belongs to this process.
            unsafe {
                TrackMouseEvent(&mut track);
            }
            Some(0)
        }
        WM_MOUSELEAVE => {
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
            Some(0)
        }
        WM_SETCURSOR if (lparam as u32 & 0xFFFF) as isize == HTCLIENT => {
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
            Some(1)
        }
        WM_LBUTTONDOWN => {
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
                // SAFETY: this message belongs to the registered native window
                // that owns the local thumb-drag state. Capture lasts only
                // until that state is cleared below.
                unsafe {
                    SetCapture(window);
                }
                invalidate(window);
            }
            Some(0)
        }
        WM_LBUTTONUP => {
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
                // SAFETY: only this window starts the matching capture above,
                // and the call merely releases the current thread's capture.
                unsafe {
                    ReleaseCapture();
                }
                return Some(0);
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
                return Some(0);
            }
            // Focus first, then invoke. A click on a field only moves focus,
            // and a click on an action does both — pressing a control is also
            // how a person expects to focus it. Treating these as alternatives
            // is what left a field unreachable by pointer.
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
                return Some(0);
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
                return Some(0);
            }
            let hit = startup_lab::action_at(
                rect.width() as f32,
                rect.height() as f32,
                point(x as f32, y as f32),
            )
            .map(|index| &startup_lab::ACTIONS[index]);
            if let Some(action) = hit
            && let Ok(Some(View::StartupLab(lab))) = registry::view_for(window)
            // Hit-testing and drawing read the same availability value, so a
            // tile drawn as planned cannot be activated by a click.
            && startup_lab::tile_is_live(action, &lab)
            {
                if action.kind == startup_lab::ActionKind::LaunchDevelopmentFixture {
                    begin_product_session(window);
                } else if let Some((title, document)) = action_document(action.kind, &lab) {
                    // A failure to open a diagnostic window is not fatal to the
                    // surface that launched it.
                    let _ = open_document_window(&title, document);
                }
            }
            Some(0)
        }
        WM_CAPTURECHANGED => {
            let _ = registry::with_ui_lab(window, ui_lab::UiLab::end_scrollbar_drag)
                .ok()
                .flatten()
                .or_else(|| {
                    registry::with_ui_session(window, |session| session.end_scrollbar_drag())
                        .ok()
                        .flatten()
                });
            Some(0)
        }
        _ => None,
    }
}
