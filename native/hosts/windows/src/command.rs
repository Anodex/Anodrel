//! Exact command-line dispatch for Windows host diagnostics and development routes.
//!
//! Startup preparation and manifest handling live in `startup`, while this
//! module accepts only the already-collected process arguments and chooses one
//! existing route. It does not parse application content or create a window.

use std::{error::Error, io, time::Instant};

use crate::{
    native_context_menu_template, native_file_binary_write_template, native_file_write_template,
    native_form_template, native_live_status_template, native_menu_template,
    native_multi_window_template, native_network_probe, native_notification_template, native_probe,
    native_scroll_window_template, native_template, native_tray_template, native_ui_probe,
    native_window_controls_template, product, sample, startup, uia_invoke_probe,
    uia_live_status_event_probe, uia_structure_event_probe, win32,
};

macro_rules! one_argument_command {
    ($arguments:expr, $flag:literal, $run:path) => {
        if let [received, value] = $arguments.as_slice()
            && received == $flag
        {
            return $run(value);
        }
    };
}

macro_rules! two_argument_command {
    ($arguments:expr, $flag:literal, $run:path) => {
        if let [received, first, second] = $arguments.as_slice()
            && received == $flag
        {
            return $run(first, second);
        }
    };
}

const USAGE: &str = concat!(
    "usage: anodrel-windows-host <exact command>\n",
    "local reports: --owned-text-report | --owned-text-comparison-report | ",
    "--idle-performance-report | ",
    "--taskbar-progress-probe | --startup-report <anodrel.application.json>\n",
    "surfaces: --application <anodrel.application.json> | ",
    "--showcase <anodrel.application.json> | --ui-preview <document.json> | ",
    "--ui-lab | --window-lab | --window-group-lab\n",
    "product and compiled/native sample routes require their documented exact ",
    "arguments; see docs/DEVELOPMENT.md and docs/DEVELOPMENT_DIAGNOSTICS.md"
);

/// Dispatches one exact host command or opens the ordinary diagnostics surface.
pub(crate) fn run(arguments: Vec<String>, started: Instant) -> Result<(), Box<dyn Error>> {
    if arguments.as_slice() == ["--taskbar-progress-probe"] {
        return win32::run_taskbar_progress_probe().map_err(Into::into);
    }
    if arguments.as_slice() == ["--idle-performance-report"] {
        return win32::run_idle_performance_report().map_err(Into::into);
    }
    if arguments.as_slice() == ["--owned-text-report"] {
        return win32::run_owned_text_report().map_err(Into::into);
    }
    if arguments.as_slice() == ["--owned-text-comparison-report"] {
        return win32::run_owned_text_comparison_report().map_err(Into::into);
    }

    one_argument_command!(arguments, "--native-sample-client", native_probe::run);
    one_argument_command!(
        arguments,
        "--native-network-sample-client",
        native_network_probe::run
    );
    one_argument_command!(arguments, "--native-ui-sample-client", native_ui_probe::run);
    one_argument_command!(arguments, "--uia-invoke-probe", uia_invoke_probe::run);
    one_argument_command!(
        arguments,
        "--uia-structure-event-probe",
        uia_structure_event_probe::run
    );
    one_argument_command!(
        arguments,
        "--uia-live-status-event-probe",
        uia_live_status_event_probe::run
    );
    one_argument_command!(arguments, "--native-template-client", native_template::run);
    one_argument_command!(
        arguments,
        "--native-form-template-client",
        native_form_template::run
    );
    one_argument_command!(
        arguments,
        "--native-live-status-template-client",
        native_live_status_template::run
    );
    one_argument_command!(
        arguments,
        "--native-menu-template-client",
        native_menu_template::run
    );
    one_argument_command!(
        arguments,
        "--native-context-menu-template-client",
        native_context_menu_template::run
    );
    one_argument_command!(
        arguments,
        "--native-tray-template-client",
        native_tray_template::run
    );
    one_argument_command!(
        arguments,
        "--native-notification-template-client",
        native_notification_template::run
    );
    one_argument_command!(
        arguments,
        "--native-file-write-template-client",
        native_file_write_template::run
    );
    one_argument_command!(
        arguments,
        "--native-file-binary-write-template-client",
        native_file_binary_write_template::run
    );
    one_argument_command!(
        arguments,
        "--native-multi-window-template-client",
        native_multi_window_template::run
    );
    one_argument_command!(
        arguments,
        "--native-scroll-window-template-client",
        native_scroll_window_template::run
    );
    one_argument_command!(
        arguments,
        "--native-window-controls-template-client",
        native_window_controls_template::run
    );

    two_argument_command!(arguments, "--sample-client", sample::run);
    two_argument_command!(arguments, "--sample-ui-client", sample::run_ui_session);
    two_argument_command!(
        arguments,
        "--sample-ui-live-status-client",
        sample::run_ui_session_with_live_status
    );
    two_argument_command!(
        arguments,
        "--sample-ui-file-client",
        sample::run_ui_session_with_open_file_dialog
    );
    two_argument_command!(
        arguments,
        "--sample-ui-folder-client",
        sample::run_ui_session_with_open_folder_dialog
    );
    two_argument_command!(
        arguments,
        "--sample-ui-folder-entries-client",
        sample::run_ui_session_with_selected_folder_entries
    );
    two_argument_command!(
        arguments,
        "--sample-ui-file-text-client",
        sample::run_ui_session_with_selected_file_text
    );
    two_argument_command!(
        arguments,
        "--sample-ui-save-client",
        sample::run_ui_session_with_save_file_dialog
    );
    two_argument_command!(
        arguments,
        "--sample-ui-file-write-client",
        sample::run_ui_session_with_selected_file_write
    );
    two_argument_command!(
        arguments,
        "--sample-ui-file-binary-write-client",
        sample::run_ui_session_with_selected_binary_file_write
    );
    two_argument_command!(
        arguments,
        "--sample-ui-storage-client",
        sample::run_ui_session_with_storage
    );
    two_argument_command!(
        arguments,
        "--sample-ui-scroll-client",
        sample::run_ui_session_with_scroll
    );
    two_argument_command!(
        arguments,
        "--sample-ui-diagnostics-client",
        sample::run_ui_session_with_diagnostics
    );
    two_argument_command!(
        arguments,
        "--sample-ui-credentials-client",
        sample::run_ui_session_with_credentials
    );
    two_argument_command!(
        arguments,
        "--sample-ui-notification-client",
        sample::run_ui_session_with_notification
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-title-client",
        sample::run_ui_session_with_window_title
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-state-client",
        sample::run_ui_session_with_window_state
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-state-read-client",
        sample::run_ui_session_with_window_state_read
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-state-changes-client",
        sample::run_ui_session_with_window_state_changes
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-focus-client",
        sample::run_ui_session_with_window_focus
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-fullscreen-client",
        sample::run_ui_session_with_window_fullscreen
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-size-client",
        sample::run_ui_session_with_window_size
    );
    two_argument_command!(
        arguments,
        "--sample-ui-window-size-fullscreen-client",
        sample::run_ui_session_with_window_size_while_fullscreen
    );
    two_argument_command!(
        arguments,
        "--sample-ui-fields-client",
        sample::run_ui_session_with_field_read
    );
    two_argument_command!(
        arguments,
        "--sample-ui-menu-client",
        sample::run_ui_session_with_menu
    );

    one_argument_command!(arguments, "--product-session", product::run);
    one_argument_command!(arguments, "--product-launch", product::run_product_launcher);
    one_argument_command!(arguments, "--application", startup::run_application_window);
    if let [received, manifest_path] = arguments.as_slice()
        && received == "--showcase"
    {
        return startup::run_startup_lab(manifest_path, started);
    }
    if arguments.as_slice() == ["--window-lab"] {
        return win32::run_window_lab().map_err(Into::into);
    }
    if arguments.as_slice() == ["--window-group-lab"] {
        return win32::run_window_group_lab().map_err(Into::into);
    }
    if arguments.as_slice() == ["--ui-lab"] {
        return win32::run_ui_lab().map_err(Into::into);
    }
    if arguments.as_slice() == ["--uia-property-probe"] {
        return win32::run_uia_property_probe().map_err(Into::into);
    }
    if arguments.as_slice() == ["--uia-focus-probe"] {
        return win32::run_uia_focus_probe().map_err(Into::into);
    }
    if arguments.as_slice() == ["--uia-focus-event-probe"] {
        return win32::run_uia_focus_event_probe().map_err(Into::into);
    }
    one_argument_command!(arguments, "--ui-preview", startup::run_ui_preview);
    if let [received, manifest_path] = arguments.as_slice()
        && received == "--startup-report"
    {
        return startup::run_startup_report(manifest_path, started);
    }
    if arguments.as_slice() == ["--crash-report-selftest"] {
        return win32::run_crash_report_selftest();
    }
    // Debug builds only. Falls through to the usage error in a release build,
    // which is the point: nothing a user runs can be asked to fault.
    #[cfg(debug_assertions)]
    if arguments.as_slice() == ["--crash-selftest-panic"] {
        return win32::run_crash_selftest_panic();
    }
    if !arguments.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, USAGE).into());
    }
    startup::run_diagnostics_window()
}
