//! Internal window creation, readiness, and message-loop lifecycle.

use super::super::*;

pub(crate) fn run_windows(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
) -> io::Result<()> {
    run_windows_after_shown(definitions, primary_instance, |_| Ok(()))
}

/// Builds fixed windows and runs one host-selected setup before they are
/// shown, so setup that waits for a shell-created window resource cannot miss
/// its first notification.
pub(crate) fn run_windows_after_created<F>(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
    after_created: F,
) -> io::Result<()>
where
    F: FnOnce(&[Hwnd]) -> io::Result<()>,
{
    run_windows_with_hooks(definitions, primary_instance, after_created, |_| Ok(()))
}

/// Builds the host's fixed windows, shows each one, and starts one
/// host-selected follow-up only after they are available to Windows clients.
///
/// This remains internal to host launch paths: callers cannot pass a native
/// handle or select a callback through the application protocol.
pub(crate) fn run_windows_after_shown<F>(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
    after_shown: F,
) -> io::Result<()>
where
    F: FnOnce(&[Hwnd]) -> io::Result<()>,
{
    run_windows_with_hooks(definitions, primary_instance, |_| Ok(()), after_shown)
}

fn run_windows_with_hooks<BeforeShown, AfterShown>(
    definitions: Vec<WindowDefinition>,
    primary_instance: Option<&PrimaryInstance>,
    before_shown: BeforeShown,
    after_shown: AfterShown,
) -> io::Result<()>
where
    BeforeShown: FnOnce(&[Hwnd]) -> io::Result<()>,
    AfterShown: FnOnce(&[Hwnd]) -> io::Result<()>,
{
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
    register_taskbar_messages();
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
    if let Err(error) = before_shown(&windows) {
        destroy_windows(&windows);
        return Err(error);
    }
    if let Some(primary_instance) = primary_instance
        && let Err(error) = primary_instance.mark_ready()
    {
        destroy_windows(&windows);
        return Err(error);
    }
    for &window in &windows {
        show_and_update(window);
    }
    if let Err(error) = after_shown(&windows) {
        destroy_windows(&windows);
        return Err(error);
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
