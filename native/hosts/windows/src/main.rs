#![deny(unsafe_op_in_unsafe_fn)]

mod development_ui_session;
mod native_context_menu_template;
mod native_form_template;
mod native_live_status_template;
mod native_menu_template;
mod native_multi_window_template;
mod native_network_probe;
mod native_probe;
mod native_scroll_window_template;
mod native_template;
mod native_ui_probe;
mod native_window_controls_template;
mod product;
mod sample;
mod session_ui;
mod uia_invoke_probe;
mod uia_structure_event_probe;
mod win32;

use std::{
    env,
    error::Error,
    fs::File,
    io::{self, Read},
    path::Path,
    time::Instant,
};

use anodrel_application::ApplicationPackage;
use anodrel_core::{CoreHost, HostPolicy};
use anodrel_protocol::{Capability, JsonValue};
use anodrel_ui::UiDocument;
use anodrel_ui_document::{MAX_ENCODED_DOCUMENT_BYTES, decode};
use anodrel_windows_instance::{InstanceClaim, InstanceScope, claim};
use anodrel_windows_pipe::run_health_self_test;

fn main() -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    // Requested before any window exists, so the surface is composed at the
    // display's real pixel density instead of being scaled up by the system.
    win32::enable_dpi_awareness();
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-sample-client"
    {
        return native_probe::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-network-sample-client"
    {
        return native_network_probe::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-ui-sample-client"
    {
        return native_ui_probe::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--uia-invoke-probe"
    {
        return uia_invoke_probe::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--uia-structure-event-probe"
    {
        return uia_structure_event_probe::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-template-client"
    {
        return native_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-form-template-client"
    {
        return native_form_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-live-status-template-client"
    {
        return native_live_status_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-menu-template-client"
    {
        return native_menu_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-context-menu-template-client"
    {
        return native_context_menu_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-multi-window-template-client"
    {
        return native_multi_window_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-scroll-window-template-client"
    {
        return native_scroll_window_template::run(client_path);
    }
    if let [command, client_path] = arguments.as_slice()
        && command == "--native-window-controls-template-client"
    {
        return native_window_controls_template::run(client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-client"
    {
        return sample::run(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-client"
    {
        return sample::run_ui_session(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-live-status-client"
    {
        return sample::run_ui_session_with_live_status(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-file-client"
    {
        return sample::run_ui_session_with_open_file_dialog(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-folder-client"
    {
        return sample::run_ui_session_with_open_folder_dialog(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-folder-entries-client"
    {
        return sample::run_ui_session_with_selected_folder_entries(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-file-text-client"
    {
        return sample::run_ui_session_with_selected_file_text(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-save-client"
    {
        return sample::run_ui_session_with_save_file_dialog(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-file-write-client"
    {
        return sample::run_ui_session_with_selected_file_write(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-file-binary-write-client"
    {
        return sample::run_ui_session_with_selected_binary_file_write(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-storage-client"
    {
        return sample::run_ui_session_with_storage(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-scroll-client"
    {
        return sample::run_ui_session_with_scroll(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-diagnostics-client"
    {
        return sample::run_ui_session_with_diagnostics(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-credentials-client"
    {
        return sample::run_ui_session_with_credentials(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-notification-client"
    {
        return sample::run_ui_session_with_notification(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-title-client"
    {
        return sample::run_ui_session_with_window_title(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-state-client"
    {
        return sample::run_ui_session_with_window_state(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-state-read-client"
    {
        return sample::run_ui_session_with_window_state_read(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-state-changes-client"
    {
        return sample::run_ui_session_with_window_state_changes(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-focus-client"
    {
        return sample::run_ui_session_with_window_focus(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-fullscreen-client"
    {
        return sample::run_ui_session_with_window_fullscreen(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-size-client"
    {
        return sample::run_ui_session_with_window_size(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-window-size-fullscreen-client"
    {
        return sample::run_ui_session_with_window_size_while_fullscreen(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-fields-client"
    {
        return sample::run_ui_session_with_field_read(node_path, client_path);
    }
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-ui-menu-client"
    {
        return sample::run_ui_session_with_menu(node_path, client_path);
    }
    if let [command, application_id] = arguments.as_slice()
        && command == "--product-session"
    {
        return product::run(application_id);
    }
    if let [command, manifest_path] = arguments.as_slice()
        && command == "--application"
    {
        return run_application_window(manifest_path);
    }
    if let [command, manifest_path] = arguments.as_slice()
        && command == "--showcase"
    {
        return run_startup_lab(manifest_path, started);
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
    if let [command, document_path] = arguments.as_slice()
        && command == "--ui-preview"
    {
        return run_ui_preview(document_path);
    }
    if let [command, manifest_path] = arguments.as_slice()
        && command == "--startup-report"
    {
        return run_startup_report(manifest_path, started);
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
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: anodrel-windows-host [--ui-lab | --uia-property-probe | --uia-focus-probe | --uia-focus-event-probe | --uia-invoke-probe <native-client.exe> | --uia-structure-event-probe <native-client.exe> | --ui-preview <document.json> | --startup-report <anodrel.application.json> | --crash-report-selftest | --window-lab | --window-group-lab | --showcase <anodrel.application.json> | --application <anodrel.application.json> | --product-session <applicationId> | --native-sample-client <native-client.exe> | --native-network-sample-client <native-client.exe> | --native-ui-sample-client <native-client.exe> | --native-template-client <native-template.exe> | --native-form-template-client <native-form-template.exe> | --native-live-status-template-client <native-live-status-template.exe> | --native-menu-template-client <native-menu-template.exe> | --native-multi-window-template-client <native-multi-window-template.exe> | --native-scroll-window-template-client <native-scroll-window-template.exe> | --native-window-controls-template-client <native-window-controls-template.exe> | --sample-client <node.exe> <native-client.js> | --sample-ui-client <node.exe> <native-client.js> | --sample-ui-live-status-client <node.exe> <native-client.js> | --sample-ui-file-client <node.exe> <native-client.js> | --sample-ui-file-text-client <node.exe> <native-client.js> | --sample-ui-save-client <node.exe> <native-client.js> | --sample-ui-file-write-client <node.exe> <native-client.js> | --sample-ui-file-binary-write-client <node.exe> <native-client.js> | --sample-ui-storage-client <node.exe> <native-client.js> | --sample-ui-scroll-client <node.exe> <native-client.js> | --sample-ui-diagnostics-client <node.exe> <native-client.js> | --sample-ui-credentials-client <node.exe> <native-client.js> | --sample-ui-window-title-client <node.exe> <native-client.js> | --sample-ui-window-state-client <node.exe> <native-client.js> | --sample-ui-window-state-read-client <node.exe> <native-client.js> | --sample-ui-window-state-changes-client <node.exe> <native-client.js> | --sample-ui-window-focus-client <node.exe> <native-client.js> | --sample-ui-window-fullscreen-client <node.exe> <native-client.js> | --sample-ui-window-size-client <node.exe> <native-client.js> | --sample-ui-window-size-fullscreen-client <node.exe> <native-client.js> | --sample-ui-fields-client <node.exe> <native-client.js> | --sample-ui-menu-client <node.exe> <native-client.js>]",
        )
        .into());
    }
    run_diagnostics_window()
}

/// Loads one bounded regular-file UI document for the explicit developer preview.
///
/// This is intentionally not an application, package, or session loader. It
/// reads only the named local file, then validates the whole document before
/// the caller creates a native window.
fn load_ui_preview_document(path: &Path) -> Result<UiDocument, Box<dyn Error>> {
    let encoded = read_bounded_regular_utf8(path)?;
    Ok(decode(&encoded)?)
}

fn run_ui_preview(document_path: &str) -> Result<(), Box<dyn Error>> {
    let document = load_ui_preview_document(Path::new(document_path))?;
    win32::run_ui_preview(document)?;
    Ok(())
}

fn read_bounded_regular_utf8(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "UI preview input must be a regular file",
        ));
    }
    if metadata.len() > MAX_ENCODED_DOCUMENT_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "UI preview input exceeds the encoded document limit",
        ));
    }

    let mut encoded = String::new();
    let mut bounded = file.take(MAX_ENCODED_DOCUMENT_BYTES as u64 + 1);
    bounded.read_to_string(&mut encoded)?;
    if encoded.len() > MAX_ENCODED_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "UI preview input exceeds the encoded document limit",
        ));
    }
    Ok(encoded)
}

fn run_diagnostics_window() -> Result<(), Box<dyn Error>> {
    let display = check_core_health()?;
    win32::run("Anodrel Windows host", &display)?;
    Ok(())
}

fn run_startup_lab(manifest_path: &str, started: Instant) -> Result<(), Box<dyn Error>> {
    let package = ApplicationPackage::load(manifest_path)?;
    let instance = match claim(
        package.identity().application_id(),
        InstanceScope::StartupLab,
    )? {
        InstanceClaim::Primary(instance) => instance,
        InstanceClaim::Existing(existing) => return Ok(existing.activate()?),
    };
    let launch = complete_startup_checks(&package)?;
    win32::run_startup_lab(
        package_facts(&package),
        &instance,
        started.elapsed(),
        launch,
    )?;
    Ok(())
}

/// Runs every check the host completes before a Startup Lab surface can open.
///
/// Shared by the surface and by `--startup-report`, so a reported startup time
/// is the time the surface actually waits for rather than a second sequence
/// that could drift from it.
fn complete_startup_checks(
    package: &ApplicationPackage,
) -> Result<product::PreflightOutcome, Box<dyn Error>> {
    // Verification only: this reads machine policy, revalidates the locked
    // executable digest, and checks Authenticode without creating a process,
    // pipe, or bootstrap material. It is the most expensive check here on a
    // provisioned machine, so it runs beside the two below rather than after
    // them. Its single answer decides whether the launch tile exists at all;
    // see `docs/PRODUCT_FIXTURE.md`.
    let preflight = product::FixturePreflight::begin();
    check_core_health()?;
    run_health_self_test(HostPolicy::new(
        package.identity().application_id(),
        vec![Capability::DiagnosticsRead],
        "anodrel-windows-host",
    )?)?;
    // Joined before the window exists: the tile's state must be resolved before
    // the surface opens, so drawing and hit-testing share one settled value.
    // The same outcome also selects the surface's launch diagnostic entry.
    Ok(preflight.finish())
}

/// Runs the startup checks, prints their readings as JSON, and exits.
///
/// Deliberately does **not** claim the single-instance mutex. A measurement
/// must not fight a running Startup Lab for it, and must not leave a claim that
/// makes the next launch think a surface is already open.
fn run_startup_report(manifest_path: &str, started: Instant) -> Result<(), Box<dyn Error>> {
    let package = ApplicationPackage::load(manifest_path)?;
    let _ = complete_startup_checks(&package)?;
    win32::print_startup_report(package.identity().application_id(), started.elapsed());
    Ok(())
}

/// Copies the display-safe facts out of a validated package.
///
/// The window layer never receives the package itself, so it cannot reach a
/// resolved filesystem path or any value that skipped validation.
fn package_facts(package: &ApplicationPackage) -> win32::PackageFacts {
    win32::PackageFacts {
        display_name: package.identity().display_name().to_owned(),
        application_id: package.identity().application_id().to_owned(),
        content_format: package.content().format().to_owned(),
        content_path: package.content().path().to_owned(),
        content_digest: package.content().digest().to_owned(),
        content_bytes: package.content().byte_length(),
    }
}

fn check_core_health() -> Result<String, Box<dyn Error>> {
    let host = CoreHost::new(HostPolicy::new(
        "anodrel.windows-host",
        vec![Capability::DiagnosticsRead],
        "anodrel-windows-host",
    )?);
    let response = host.handle_json(
        r#"{"protocolVersion":{"major":1,"minor":0},"kind":"request","requestId":"startup-health","operation":"platform.health","payload":{}}"#,
    );
    health_display(&response)
}

fn run_application_window(manifest_path: &str) -> Result<(), Box<dyn Error>> {
    let package = ApplicationPackage::load(manifest_path)?;
    let instance = match claim(
        package.identity().application_id(),
        InstanceScope::Application,
    )? {
        InstanceClaim::Primary(instance) => instance,
        InstanceClaim::Existing(existing) => return Ok(existing.activate()?),
    };
    let title = format!("Anodrel - {}", package.identity().display_name());
    let subtitle = format!(
        "Verified package  ·  {}",
        package.identity().application_id()
    );
    win32::run_application(&title, &subtitle, package.text(), &instance)?;
    Ok(())
}

fn health_display(response: &str) -> Result<String, Box<dyn Error>> {
    let response = JsonValue::parse(response)?;
    let fields = response
        .as_object()
        .ok_or_else(|| io::Error::other("health response is not an object"))?;
    let status = string_field(fields, "status")?;
    let result = fields
        .get("result")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| io::Error::other("health response has no result"))?;
    let host_name = string_field(result, "hostName")?;
    let protocol_version = result
        .get("protocolVersion")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| io::Error::other("health response has no protocol version"))?;
    let major = protocol_version
        .get("major")
        .and_then(JsonValue::as_u16)
        .ok_or_else(|| io::Error::other("health response major version is invalid"))?;
    let minor = protocol_version
        .get("minor")
        .and_then(JsonValue::as_u16)
        .ok_or_else(|| io::Error::other("health response minor version is invalid"))?;

    Ok(format!(
        "Anodrel direct Windows host\n\nThe window, message loop, UTF-16 conversion, drawing, JSON codec, and protocol core are built into Anodrel.\n\nStartup protocol check\nstatus: {status}\nhost: {host_name}\nprotocol: {major}.{minor}"
    ))
}

fn string_field<'a>(
    fields: &'a std::collections::BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<&'a str, io::Error> {
    fields
        .get(field)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| io::Error::other(format!("health response {field} is invalid")))
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
