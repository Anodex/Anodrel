//! Contained Win32 message dispatch for registered Anodrel windows.
//!
//! The dispatcher works only with host-owned window records. It routes native
//! messages into fixed surface behavior and private service mailboxes; it never
//! accepts a window handle or a native command from application protocol data.

use super::*;

/// Runs one window-procedure body, reporting `None` if it panicked.
///
/// A panic must not leave this function. `window_proc` is `extern "system"`,
/// which does not unwind, so Rust turns an escaping panic into an immediate
/// process abort — and an abort runs no destructor. That would leave a verified
/// product child running with no host, and a notification-area entry on screen
/// with nothing behind it.
///
/// The payload is dropped here rather than inspected. A panic message can carry
/// arbitrary values, and nothing derived from one may reach a protocol
/// response, the diagnostic ledger, a crash record, or an application.
pub(super) fn contain_panic<R>(body: impl FnOnce() -> R) -> Option<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).ok()
}

pub(super) unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: Uint,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    // SAFETY: the dispatch body keeps the same contract this callback always
    // had; only its unwinding behaviour changes.
    match contain_panic(|| unsafe { dispatch(window, message, wparam, lparam) }) {
        Some(result) => result,
        None => {
            // Leave evidence before leaving. Containment on its own makes a
            // defect look exactly like a clean exit, so the record is written
            // here rather than after the loop, while the window that was being
            // served is still known.
            crash::report_contained_panic(window);
            // Fail closed but orderly. Ending the message loop lets `run_windows`
            // return and drop every registered view, which shuts down a running
            // product child and removes any notification entry — the cleanup an
            // abort would have skipped entirely.
            // SAFETY: posting a quit message is valid from a window procedure.
            unsafe { PostQuitMessage(1) };
            0
        }
    }
}

unsafe fn dispatch(window: Hwnd, message: Uint, wparam: Wparam, lparam: Lparam) -> Lresult {
    // Answered before the match below so an unrelated object request still
    // reaches the default procedure. It publishes semantics outward and only an
    // enabled authenticated-session button can offer its existing revision-bound
    // action mailbox. Focus reporting is a copied layout snapshot; neither
    // feature has a native-input or application callback route. Field values
    // are another copied snapshot and remain read-only to automation.
    if message == WM_GETOBJECT {
        let publication = accessible_elements_for(window);
        // SAFETY: this window belongs to the current thread's message queue,
        // which is the only thread that dispatches to this procedure.
        if let Some(result) =
            unsafe { anodrel_windows_uia::answer_get_object(window, wparam, lparam, publication) }
        {
            return result;
        }
    }
    if message == WM_ANODREL_UIA_FOCUS {
        service_accessibility_focus(window);
        return 0;
    }
    if message == WM_ANODREL_UIA_SCROLL {
        service_accessibility_scroll(window);
        return 0;
    }
    if message == WM_ANODREL_NOTIFICATION_AREA && tray::handle_callback(window, lparam) {
        return 0;
    }
    if message == WM_SYSCOMMAND && is_product_update_command(wparam) {
        start_product_update(window);
        return 0;
    }
    if let Some(result) = super::input::handle_input_message(window, message, wparam, lparam) {
        return result;
    }
    match message {
        message
            if ACTIVATION_MESSAGE
                .get()
                .is_some_and(|expected| *expected == message) =>
        {
            // SAFETY: the activation message carries no data and this window is
            // created by the current process. Windows remains authoritative over
            // whether the foreground request is honored.
            unsafe {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
            0
        }
        // The canvas covers every pixel, so erasing first would only flash.
        WM_ERASEBKGND => 1,
        WM_GETMINMAXINFO => {
            let (width, height) = window_size_for_client(MIN_CLIENT_WIDTH, MIN_CLIENT_HEIGHT);
            // SAFETY: for this message lparam points to a writable MINMAXINFO
            // supplied by the system.
            unsafe {
                let info = &mut *(lparam as *mut MinMaxInfo);
                info.minTrackSize = Point {
                    x: width,
                    y: height,
                };
            }
            0
        }
        WM_PAINT => {
            #[cfg(debug_assertions)]
            crash_selftest::fault_if_armed();
            let mut paint_struct = PaintStruct::default();
            // SAFETY: Windows calls this procedure for a valid window, and
            // paint_struct is writable stack storage for the matching EndPaint.
            let device_context = unsafe { BeginPaint(window, &mut paint_struct) };
            if device_context != 0 {
                if let Ok(Some(view)) = registry::view_for(window) {
                    let micros = paint(window, device_context, &view, paint_struct.rcPaint);
                    let _ = registry::with_startup_lab(window, |lab| {
                        lab.last_frame_micros = micros;
                    });
                }
                // SAFETY: BeginPaint initialized paint_struct for this window.
                unsafe {
                    EndPaint(window, &paint_struct);
                }
            }
            0
        }
        WM_TIMER if wparam == REVEAL_TIMER => {
            let state = registry::with_startup_lab(window, |lab| {
                let elapsed = lab.revealed_at.elapsed().as_millis() as u64;
                let settling = !lab.ambient && elapsed >= startup_lab::REVEAL_MILLIS;
                if settling {
                    lab.ambient = true;
                }
                (elapsed, lab.ambient, settling)
            })
            .ok()
            .flatten();
            let Some((_, ambient, settling)) = state else {
                // Not an animated surface any more; stop waking for it.
                // SAFETY: killing a timer that does not exist is a no-op.
                unsafe { KillTimer(window, REVEAL_TIMER) };
                return 0;
            };

            if settling {
                // The reveal is done. Drop from the reveal's frame rate to the
                // ambient one rather than stopping: the mark keeps breathing,
                // but at a cadence a settled screen can afford.
                // SAFETY: re-setting an existing timer changes its interval.
                unsafe {
                    SetTimer(window, REVEAL_TIMER, AMBIENT_INTERVAL_MILLIS, 0);
                }
            }

            let rect = client_rect(window);
            match ambient
                .then(|| startup_lab::ambient_region(rect.width() as f32, rect.height() as f32))
                .flatten()
            {
                // The mark moves, and its translucent foreground detail band
                // is redrawn above it. Both fit inside this bounded region.
                Some(region) => invalidate_region(window, region),
                None => invalidate(window),
            }
            0
        }
        WM_TIMER if wparam == UI_SESSION_TIMER => {
            if let Some(poll) = registry::poll_ui_session(window).ok().flatten() {
                if poll.close_requested {
                    // SAFETY: the request is consumed only by this window's UI
                    // thread, which owns the host-created native handle.
                    unsafe { DestroyWindow(window) };
                    return 0;
                }
                if poll.document_changed {
                    invalidate(window);
                    raise_accessibility_structure_changed(window);
                    if let Some(status) = poll.changed_status.as_ref() {
                        raise_accessibility_live_region_changed(window, status);
                    }
                }
            }
            service_session_window_open(window);
            service_session_window_close(window);
            service_notification(window);
            service_tray(window);
            service_menu(window);
            service_context_menu(window);
            service_window_title(window);
            service_window_state(window);
            service_window_state_read(window);
            service_window_focus(window);
            service_window_fullscreen(window);
            service_window_size(window);
            service_field_read(window);
            service_product_update(window);
            if let Ok(Some(request)) = registry::take_file_dialog_request(window) {
                let selection = match request.kind() {
                    FileDialogRequestKind::OpenFolder => {
                        anodrel_windows_file_dialog::open_folder_with_owner(window).map(|path| {
                            path.map_or(FileDialogSelection::Cancelled, FileDialogSelection::Folder)
                        })
                    }
                    FileDialogRequestKind::OpenFolderWithReference => {
                        let folder_entries = registry::folder_entry_service(window).ok().flatten();
                        match folder_entries {
                            Some(folder_entries) => {
                                anodrel_windows_file_dialog::open_folder_with_owner_and_capture(
                                    window,
                                    |path| {
                                        let folder =
                                            anodrel_windows_folder_access::open_selected_folder(
                                                path,
                                            )
                                            .map_err(|_| ())?;
                                        folder_entries.register(folder).map_err(|_| ())
                                    },
                                )
                                .map(|selection| {
                                    selection.map_or(
                                        FileDialogSelection::Cancelled,
                                        |(path, reference)| {
                                            FileDialogSelection::CapturedFolder(path, reference)
                                        },
                                    )
                                })
                            }
                            None => Err(anodrel_windows_file_dialog::FileDialogError::Unavailable),
                        }
                    }
                    FileDialogRequestKind::Open => {
                        anodrel_windows_file_dialog::open_file_with_owner(window, request.filters())
                            .map(|path| {
                                path.map_or(
                                    FileDialogSelection::Cancelled,
                                    FileDialogSelection::Selected,
                                )
                            })
                    }
                    FileDialogRequestKind::Save => {
                        anodrel_windows_file_dialog::save_file_with_owner(window, request.filters())
                            .map(|path| {
                                path.map_or(
                                    FileDialogSelection::Cancelled,
                                    FileDialogSelection::Saved,
                                )
                            })
                    }
                    FileDialogRequestKind::OpenWithReference => {
                        let file_text = registry::file_text_service(window).ok().flatten();
                        match file_text {
                            Some(file_text) => {
                                anodrel_windows_file_dialog::open_file_with_owner_and_capture(
                                    window,
                                    request.filters(),
                                    |path| {
                                        let file =
                                            anodrel_windows_file_access::open_selected_file(path)
                                                .map_err(|_| ())?;
                                        file_text.register(file).map_err(|_| ())
                                    },
                                )
                                .map(|selection| {
                                    selection.map_or(
                                        FileDialogSelection::Cancelled,
                                        |(path, reference)| {
                                            FileDialogSelection::Captured(path, reference)
                                        },
                                    )
                                })
                            }
                            None => Err(anodrel_windows_file_dialog::FileDialogError::Unavailable),
                        }
                    }
                    FileDialogRequestKind::SaveWithReference => {
                        let file_text = registry::file_text_service(window).ok().flatten();
                        match file_text {
                            Some(file_text) => {
                                let file_write = file_text.write_service();
                                anodrel_windows_file_dialog::save_file_with_owner_and_capture(
                                    window,
                                    request.filters(),
                                    |path| {
                                        let file =
                                            anodrel_windows_file_access::open_save_file(path)
                                                .map_err(|_| ())?;
                                        file_write.register(file).map_err(|_| ())
                                    },
                                )
                                .map(|selection| {
                                    selection.map_or(
                                        FileDialogSelection::Cancelled,
                                        |(path, reference)| {
                                            FileDialogSelection::CapturedSave(path, reference)
                                        },
                                    )
                                })
                            }
                            None => Err(anodrel_windows_file_dialog::FileDialogError::Unavailable),
                        }
                    }
                };
                let _ = registry::complete_file_dialog_request(window, request.id(), selection);
            }
            0
        }
        WM_ACTIVATE => {
            set_ambient_running(window, (wparam & 0xFFFF) != WA_INACTIVE);
            0
        }
        WM_COMMAND => {
            let handled = registry::offer_menu_command(window, wparam, lparam)
                .ok()
                .flatten()
                .unwrap_or(false);
            if handled {
                // A current host-private normal menu command is now a bounded
                // semantic candidate. The application receives it only through
                // the authenticated pull path, never this native message.
                0
            } else {
                // SAFETY: unknown commands, accelerators, and controls retain
                // documented default Win32 handling unchanged.
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
        }
        WM_CONTEXTMENU => {
            let outcome = registry::context_menu(window)
                .ok()
                .flatten()
                .and_then(|menu| menu.show_for_pointer(window, lparam));
            match outcome {
                Some(context_menu::ContextMenuDisplay::Selected(candidate)) => {
                    let _ = registry::offer_context_menu_candidate(window, candidate);
                    0
                }
                Some(context_menu::ContextMenuDisplay::Dismissed) => 0,
                None => {
                    // Keyboard-originated context menus and views without a
                    // retained model keep documented Win32 processing.
                    unsafe { DefWindowProcW(window, message, wparam, lparam) }
                }
            }
        }
        WM_SETTINGCHANGE => {
            // Interactive native UI paints read the small direct Windows
            // appearance adapter. A system broadcast therefore schedules one
            // repaint without retaining settings or adding an application
            // observer/subscription surface.
            if registry::uses_system_appearance(window).unwrap_or(false) {
                invalidate(window);
                return 0;
            }
            // SAFETY: unhandled system settings messages retain standard
            // default Win32 processing for every other host view.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
        WM_SIZE => {
            set_ambient_running(window, wparam != SIZE_MINIMIZED);
            record_window_state_change(window);
            let rect = client_rect(window);
            let _ = registry::with_ui_lab(window, |lab| {
                lab.clamp_scroll_offsets(rect.width() as f32, rect.height() as f32);
            });
            let _ = registry::with_ui_session(window, |session| {
                session.clamp_scroll_offsets(rect.width() as f32, rect.height() as f32);
            });
            0
        }
        WM_ANODREL_PRODUCT_SESSION => {
            match product_tile::take_started() {
                Some(session) => {
                    if open_product_session_window(session).is_err() {
                        // The session is dropped by the failed call, which
                        // requests its own shutdown. Release the guard so the
                        // tile can be tried again.
                        product_tile::release();
                    }
                }
                // A start that produced nothing has already released its guard
                // and reports no reason: a verified launch can fail for causes
                // this surface must not describe.
                None => product_tile::release(),
            }
            0
        }
        WM_DPICHANGED => {
            // SAFETY: for this message lparam points to a RECT the system has
            // sized for the new DPI.
            let suggested = unsafe { *(lparam as *const Rect) };
            // SAFETY: the window belongs to this process; z-order and
            // activation are left untouched.
            unsafe {
                SetWindowPos(
                    window,
                    0,
                    suggested.left,
                    suggested.top,
                    suggested.width(),
                    suggested.height(),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            invalidate(window);
            0
        }
        WM_DESTROY => {
            // SAFETY: killing a timer for a window that has no session poll is
            // a no-op, and this window is being destroyed by the current UI
            // thread.
            unsafe { KillTimer(window, UI_SESSION_TIMER) };
            // Removing the view drops this window's product session, if it owns
            // one, which shuts down its child and joins both workers before the
            // guard is released. Shutdown precedes those joins, so this stays a
            // brief call rather than a wait on user-paced work.
            let removed = registry::remove(window);
            product_tile::note_destroyed(window);
            if removed.is_ok_and(|remaining| remaining == 0) {
                // SAFETY: this only posts a quit message after the final
                // native top-level window is being destroyed.
                unsafe { PostQuitMessage(0) };
            }
            0
        }
        _ => {
            // SAFETY: all unhandled messages are forwarded unchanged to the
            // documented default Win32 procedure.
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
    }
}
