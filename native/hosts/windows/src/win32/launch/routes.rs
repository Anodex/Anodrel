//! Fixed public launch routes for host-owned Windows surfaces.

use super::super::*;
use super::{lifecycle::run_windows_after_shown, startup_log_book};

/// Opens the simple native document surface.
pub fn run(title: &str, text: &str) -> io::Result<()> {
    super::lifecycle::run_windows(
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
    super::lifecycle::run_windows(
        vec![WindowDefinition::document(title, subtitle, text, 920, 580)],
        Some(instance),
    )
}

/// Opens the branded Startup Lab after the caller has completed its checks.
///
/// `startup` is the time from process start to this call, which the surface
/// reports as its startup reading. `launch` comes from the caller's
/// verification-only preflight; one outcome controls both the tile's state and
/// its diagnostic history.
pub fn run_startup_lab(
    package: PackageFacts,
    instance: &PrimaryInstance,
    startup: std::time::Duration,
    launch: PreflightOutcome,
) -> io::Result<()> {
    let scale = primary_scale();
    let launch_available = launch.allows_launch();
    super::lifecycle::run_windows(
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
    super::lifecycle::run_windows(
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
/// The worker uses the same portable handoff Protocol 1.25 uses, while the host
/// supplies both fixed captions and documents. Nothing in this route accepts
/// application data, grants, native handles, or a product child; it exists only
/// for the manual lifecycle check in `docs/WINDOW_LIFECYCLE.md`.
pub fn run_window_group_lab() -> io::Result<()> {
    window_group_lab::run()
}

/// Opens the host-owned visual and input test for the native UI foundation.
///
/// The screen uses a fixed document compiled into the host. Its action events
/// update only a visible diagnostic line; they never reach an application or a
/// native capability boundary.
pub fn run_ui_lab() -> io::Result<()> {
    super::lifecycle::run_windows(vec![ui_lab_window()], None)
}

/// Verifies the UI Lab's published Windows UI Automation property tree.
///
/// The test window and its client result both remain host-private. This route
/// has no application input and prints a fixed success message only after the
/// temporary window closes. Its documented scope is intentionally limited to
/// read-only properties and direct raw-view relationships.
pub fn run_uia_property_probe() -> io::Result<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    run_windows_after_shown(vec![ui_lab_window()], None, move |windows| {
        prioritize_uia_point_probe(windows[0])?;
        uia_property_probe::spawn(windows[0], sender)
    })?;
    match receiver.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => {
            println!("UI Automation property probe passed.");
            Ok(())
        }
        Ok(Err(_)) => Err(io::Error::other(
            "UI Automation property probe did not observe the expected fixed tree",
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "UI Automation property probe did not report a result",
        )),
    }
}

/// Keeps the temporary UI Automation geometry probe above ordinary windows.
///
/// Windows resolves `ElementFromPoint` against the topmost desktop element,
/// not against a selected host window. The test therefore uses the topmost band
/// only for its short-lived private UI Lab window, then destroys that window
/// once the read-only probe has completed. It never leaves a product window
/// topmost or changes another process's window state.
fn prioritize_uia_point_probe(window: Hwnd) -> io::Result<()> {
    // SAFETY: `window` is a visible host-created temporary diagnostic window,
    // and the flags preserve its size, position, and activation state.
    let moved = unsafe {
        SetWindowPos(
            window,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(crate) fn ui_lab_window() -> WindowDefinition {
    let scale = primary_scale();
    WindowDefinition {
        title: "Anodrel UI Lab".to_owned(),
        width: (920.0 * scale) as i32,
        height: (660.0 * scale) as i32,
        view: View::UiLab(ui_lab::UiLab::new()),
    }
}

/// Opens one operator-validated UI document as a local developer preview.
///
/// The caller must decode the document before this function is called. This
/// view has no package, protocol, application session, or capability binding;
/// its semantic actions remain local to the preview view.
pub fn run_ui_preview(document: UiDocument) -> io::Result<()> {
    let scale = primary_scale();
    super::lifecycle::run_windows(
        vec![WindowDefinition {
            title: "Anodrel UI Preview".to_owned(),
            width: (920.0 * scale) as i32,
            height: (660.0 * scale) as i32,
            view: View::UiLab(ui_lab::UiLab::preview(document)),
        }],
        None,
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
    super::lifecycle::run_windows(
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
