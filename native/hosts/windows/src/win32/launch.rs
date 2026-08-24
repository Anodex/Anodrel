//! Public host launch paths and common message-loop setup.
//!
//! These fixed routes compose host-selected views and resources. They do not
//! form a general native window API and accept no application-provided handle,
//! style, or desktop-control value.

use super::*;
use anodrel_diagnostics::Event;

/// Builds the diagnostic history displayed by the Startup Lab.
///
/// Its only input is one member of the closed event catalogue, chosen by the
/// caller's preflight. The displayed history therefore still reflects fixed host
/// milestones rather than application text, operating-system errors, paths, or
/// arbitrary caller data.
pub(super) fn startup_log_book(launch_event: Event) -> LogBook {
    let mut log = LogBook::new();
    // Chronological: the preflight runs alongside the two checks above it and is
    // settled before this surface is authorized to open.
    for event in [
        Event::PackageVerified,
        Event::CoreHealthChecked,
        Event::PipeLoopbackChecked,
        launch_event,
        Event::StartupLabAuthorized,
    ] {
        log.record(event)
            .expect("five fixed startup events fit in the diagnostic log");
    }
    log
}

/// Opens the simple native document surface.
pub fn run(title: &str, text: &str) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition::document(
            title,
            "Native surface",
            text,
            920,
            580,
        )],
        None,
    )
}

/// Opens the validated application text surface as the primary host instance.
pub fn run_application(
    title: &str,
    subtitle: &str,
    text: &str,
    instance: &PrimaryInstance,
) -> io::Result<()> {
    run_windows(
        vec![WindowDefinition::document(title, subtitle, text, 920, 580)],
        Some(instance),
    )
}

/// Opens the branded Startup Lab after the caller has completed its checks.
///
/// `startup` is the time from process start to this call, which the surface
/// reports as its startup reading.
/// `launch` must come from the caller's verification-only preflight. The surface
/// never runs that check itself, so drawing code cannot decide that a launch is
/// possible. One outcome drives both the tile's availability and its diagnostic
/// entry, so the two can never disagree.
pub fn run_startup_lab(
    package: PackageFacts,
    instance: &PrimaryInstance,
    startup: std::time::Duration,
    launch: PreflightOutcome,
) -> io::Result<()> {
    let scale = primary_scale();
    let launch_available = launch.allows_launch();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel Startup Lab".to_owned(),
            width: (startup_lab::BASE_WIDTH * scale) as i32,
            height: (startup_lab::BASE_HEIGHT * scale) as i32,
            view: View::StartupLab(StartupLab {
                package,
                log: startup_log_book(launch.event()),
                startup_millis: startup.as_millis() as u64,
                working_set_bytes: stats::working_set_bytes(),
                last_frame_micros: 0,
                revealed_at: Instant::now(),
                hovered: None,
                ambient: false,
                launch_available,
            }),
        }],
        Some(instance),
    )
}

/// Opens two static native windows to exercise the multi-window lifecycle.
pub fn run_window_lab() -> io::Result<()> {
    run_windows(
        vec![
            WindowDefinition::document(
                "Anodrel Window Lab - Primary",
                "Multi-window lifecycle",
                "This primary window proves that the direct host can keep multiple native windows alive on one User32 message loop.\n\nClose this window first: the companion window remains available. Close the final window to end the host process.",
                760,
                460,
            ),
            WindowDefinition::document(
                "Anodrel Window Lab - Companion",
                "Multi-window lifecycle",
                "This companion has its own immutable host view. Its paint messages are routed by the real Win32 window handle, not a process-global surface.\n\nClose either window in any order. The host exits only after the last one closes.",
                760,
                460,
            ),
        ],
        None,
    )
}

/// Opens a host-controlled diagnostic for the dynamic session-window path.
///
/// The worker uses the same portable handoff Protocol 1.25 uses,
/// while the host supplies both fixed captions and documents. Nothing in this
/// route accepts application data, grants, native handles, or a product child;
/// it exists solely for the manual lifecycle check in `docs/WINDOW_LIFECYCLE.md`.
pub fn run_window_group_lab() -> io::Result<()> {
    window_group_lab::run()
}

/// Deliberate fault injection, for proving the containment path end to end.
///
/// A crash reporter that nobody can trigger is one nobody knows still works.
/// The `--crash-report-selftest` route proves the store can write; this proves
/// the part that matters more — that a panic raised inside a real window
/// message is contained, classified, recorded, and shut down cleanly instead of
/// aborting the process.
///
/// Compiled only in a debug build. `start.bat` and every shipped path build in
/// release, so this cannot exist in a binary a user runs.
#[cfg(debug_assertions)]
pub(super) mod crash_selftest {
    use std::sync::atomic::{AtomicBool, Ordering};

    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Arms the next window paint to panic.
    pub(crate) fn arm() {
        ARMED.store(true, Ordering::Release);
    }

    /// Panics once if armed, disarming itself first.
    ///
    /// Disarming before panicking matters: the host repaints while shutting
    /// down, and a fault that re-armed itself would panic again inside the
    /// cleanup this route exists to observe.
    pub(crate) fn fault_if_armed() {
        if ARMED.swap(false, Ordering::AcqRel) {
            panic!("deliberate fault injected by --crash-selftest-panic");
        }
    }
}

/// Opens the UI Lab and panics inside its first paint, then reports the result.
///
/// Available in debug builds only. What to look for afterwards: the process
/// exits without aborting, and a new record appears in the location named by
/// `docs/CRASH_REPORTS.md` with `site=window-procedure` and `surface=ui-lab` —
/// the surface being the proof that classification ran against a live window
/// rather than falling back to `unknown`.
#[cfg(debug_assertions)]
pub fn run_crash_selftest_panic() -> Result<(), Box<dyn std::error::Error>> {
    crash_selftest::arm();
    run_ui_lab()?;
    println!(
        "The injected fault was contained and the host shut down. \
         Check for a record with surface=ui-lab; see docs/CRASH_REPORTS.md."
    );
    Ok(())
}

/// Runs the real startup sequence, prints its readings, and exits.
///
/// The Startup Lab already shows a startup time and a working set in its
/// footer. This is the same measurement in a form a script can keep, so a
/// startup or memory figure can be recorded across builds instead of read off a
/// screenshot.
///
/// # What it measures, and what it does not
///
/// The elapsed time covers everything the host does before a surface could
/// open: package verification, the core health check, the private pipe
/// loopback, and the launch preflight. It stops there. **It does not include
/// creating the window or its first paint**, so it is a floor for cold start
/// and must never be quoted as time-to-first-frame. See `docs/PERFORMANCE.md`.
///
/// No window opens, so the memory readings are the host's own cost with nothing
/// rendered — also a floor, and the honest one to compare against another
/// runtime's idle process.
pub fn print_startup_report(application_id: &str, elapsed: std::time::Duration) {
    let memory = stats::memory_readings();
    // A local tooling format, shaped like the performance lab's report so both
    // can be retained the same way. It is not a protocol and has no reader here.
    println!(
        concat!(
            "{{\"benchmark\":\"anodrel.host.startup.v1\",",
            "\"startupMicroseconds\":{},",
            "\"workingSetBytes\":{},\"privateBytes\":{},",
            "\"applicationId\":\"{}\",",
            "\"scope\":\"host startup checks only; no window creation, no first paint\"}}"
        ),
        elapsed.as_micros(),
        memory.working_set_bytes,
        memory.private_bytes,
        application_id,
    );
}

/// Writes one crash record through the ordinary reporting path, then exits.
///
/// A crash record is only useful if it is actually written on the machine it is
/// meant to help, and the path that writes it is reached exactly once, during a
/// defect, when nobody is watching. This is how an operator confirms the
/// location, permissions, and format without waiting for a real failure.
///
/// It records the same `window-procedure` site a contained panic does, with no
/// window registered, so the surface is `unknown`. It opens no window and
/// prints only whether the record was written — never the location, which is in
/// `docs/CRASH_REPORTS.md` for a person to look up rather than for a process to
/// hand out.
pub fn run_crash_report_selftest() -> Result<(), Box<dyn std::error::Error>> {
    match crash::report(CrashSite::WindowProcedure, CrashSurface::Unknown) {
        Some(sequence) => {
            println!("Wrote crash record {sequence}. See docs/CRASH_REPORTS.md for its location.");
            Ok(())
        }
        None => Err(io::Error::other("the host could not write a crash record").into()),
    }
}

/// Opens the host-owned visual and input test for the native UI foundation.
///
/// The screen uses a fixed document compiled into the host. Its action events
/// update only a visible diagnostic line; they never reach an application or a
/// native capability boundary.
pub fn run_ui_lab() -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel UI Lab".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiLab(ui_lab::UiLab::new()),
        }],
        None,
    )
}

/// Opens one operator-validated UI document as a local developer preview.
///
/// The caller must decode the document before this function is called. This
/// view has no package, protocol, application session, or capability binding;
/// its semantic actions remain local to the preview view.
pub fn run_ui_preview(document: UiDocument) -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: "Anodrel UI Preview".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiLab(ui_lab::UiLab::preview(document)),
        }],
        None,
    )
}

/// Opens one host-controlled native view that consumes exactly one authenticated
/// session's mailboxes. Actions enter only the bounded semantic-input mailbox
/// and remain incapable of native operations in this diagnostic.
#[allow(clippy::too_many_arguments)]
pub fn run_ui_session(
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    run_authenticated_ui_session(
        "Anodrel UI Session Lab",
        mailbox,
        input_mailbox,
        close_signal,
        file_dialog_mailbox,
        file_text,
        notifications,
        menu,
        window_title,
        window_state,
        window_focus,
        window_fullscreen,
        window_size,
        display_name,
        field_reads,
    )
}

/// Opens one host-controlled primary view for a grouped development session.
///
/// This is a fixed host route, not a generic view constructor. Its portable
/// group already contains the same primary document and input mailboxes that
/// the authenticated pipe core uses. The host creates the private native group
/// before the primary view is registered, so later `window.open` requests use
/// the identical Windows lifecycle as a verified product session without
/// giving the selected development child any native control.
pub fn run_grouped_ui_session(
    window_group: UiWindowGroup<WindowTitleProposal>,
    close_signal: SessionCloseSignal,
    title: &str,
) -> io::Result<()> {
    let primary_id = UiWindowId::primary();
    let resources = window_group.resources(&primary_id).ok_or_else(|| {
        io::Error::other("grouped development session has no primary view resources")
    })?;
    let group = session_window_group::SessionWindowGroup::new(
        window_group,
        close_signal.clone(),
        Some(title.to_owned()),
    );
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: title.to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiSession(Box::new(ui_session_view::UiSessionView::for_group_primary(
                resources,
                close_signal,
                group.member(primary_id),
            ))),
        }],
        None,
    )
}

/// Opens one host-selected authenticated application session window.
///
/// The caller must supply resources created together for one already
/// authenticated session. This is host lifecycle code, not an application
/// window-management API: the application cannot create a window, pass a
/// handle, or attach a different session's resource.
///
/// `title` is the caption the host opens with. A session holding the
/// `window.title` grant may later propose a replacement, which the host
/// composes with `display_name` before applying — the application supplies one
/// half and never the other. See `docs/WINDOW_TITLE.md`.
///
/// `window_state` carries only minimise, maximise, and restore requests for
/// this same host-selected window. It is a closed command bridge, not a native
/// handle or a window-management API; see `docs/WINDOW_STATE.md`.
///
/// `window_focus` carries only a request to foreground this same host-selected
/// window. It exposes no target, input, retry, or observed focus state; see
/// `docs/WINDOW_FOCUS.md`.
///
/// `window_fullscreen` carries only a reversible borderless or windowed mode
/// for this same host-selected window. The UI thread retains native restoration
/// facts privately; see `docs/WINDOW_FULLSCREEN.md`.
///
/// `window_size` carries only bounded logical client dimensions for this same
/// host-selected window. The UI thread derives its native frame privately; see
/// `docs/WINDOW_SIZE.md`.
#[allow(clippy::too_many_arguments)]
pub fn run_authenticated_ui_session(
    title: &str,
    mailbox: UiDocumentMailbox,
    input_mailbox: UiInputMailbox,
    close_signal: SessionCloseSignal,
    file_dialog_mailbox: FileDialogMailbox,
    file_text: WindowsFileTextService,
    notifications: NotificationMailbox,
    menu: MenuMailbox,
    window_title: WindowTitleMailbox,
    window_state: WindowStateMailbox,
    window_focus: WindowFocusMailbox,
    window_fullscreen: WindowFullscreenMailbox,
    window_size: WindowSizeMailbox,
    display_name: &str,
    field_reads: UiFieldMailbox,
) -> io::Result<()> {
    let scale = primary_scale();
    run_windows(
        vec![WindowDefinition {
            title: title.to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiSession(Box::new(
                ui_session_view::UiSessionView::new(
                    mailbox,
                    input_mailbox,
                    close_signal,
                    file_dialog_mailbox,
                    file_text,
                    notifications,
                )
                .with_menu(menu)
                .with_window_title(window_title, display_name)
                .with_window_state(window_state)
                .with_window_focus(window_focus)
                .with_window_fullscreen(window_fullscreen)
                .with_window_size(window_size)
                .with_field_reads(field_reads),
            )),
        }],
        None,
    )
}

pub(super) fn run_windows(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
) -> io::Result<()> {
    if definitions.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "host requires at least one window",
        ));
    }
    if let Some(primary_instance) = primary_instance {
        ACTIVATION_MESSAGE
            .set(primary_instance.activation_message())
            .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "activation message set"))?;
    }
    let instance = module_handle()?;
    let class_name = to_wide_null("Anodrel.DirectWindowsHost");
    ensure_window_class(instance, &class_name)?;
    let mut windows = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let animated = matches!(definition.view, View::StartupLab(_));
        let session_driven = matches!(definition.view, View::UiSession(_));
        let group_driven = definition.view.requires_group_registration();
        let window = match create_window(instance, &class_name, &definition) {
            Ok(window) => window,
            Err(error) => {
                destroy_windows(&windows);
                return Err(error);
            }
        };
        if let Err(error) = registry::insert(window, definition.view) {
            destroy_window(window);
            destroy_windows(&windows);
            return Err(error);
        }
        if group_driven {
            let joined_group = match registry::register_ui_session_window(window) {
                Ok(Some(joined)) => joined,
                Ok(None) => false,
                Err(error) => {
                    destroy_window(window);
                    destroy_windows(&windows);
                    return Err(error);
                }
            };
            if !joined_group {
                destroy_window(window);
                destroy_windows(&windows);
                return Err(io::Error::other(
                    "session window could not join its native view group",
                ));
            }
        }
        apply_icons(window);
        if animated {
            // SAFETY: the window was just created and belongs to this thread's
            // message queue. The timer is stopped when the reveal completes.
            unsafe {
                SetTimer(window, REVEAL_TIMER, REVEAL_INTERVAL_MILLIS, 0);
            }
        }
        if session_driven {
            // SAFETY: this window owns the mailbox consumer and stops this
            // low-frequency poll when the window is destroyed.
            unsafe {
                SetTimer(window, UI_SESSION_TIMER, UI_SESSION_POLL_INTERVAL_MILLIS, 0);
            }
        }
        windows.push(window);
    }
    if let Some(primary_instance) = primary_instance
        && let Err(error) = primary_instance.mark_ready()
    {
        destroy_windows(&windows);
        return Err(error);
    }
    for window in windows {
        show_and_update(window);
    }
    let result = message_loop();
    // The loop normally ends only after the last window is destroyed, but a
    // contained panic ends it while views are still registered. Dropping them
    // here shuts down anything they own; the registry is a static and would
    // otherwise never be dropped at all.
    let _ = registry::clear();
    // No window can now collect a session that finished starting during
    // shutdown either, because a posted message is only delivered while the
    // loop runs.
    product_tile::discard();
    result
}
