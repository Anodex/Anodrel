#![deny(unsafe_op_in_unsafe_fn)]

mod sample;
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
        && command == "--sample-ui-file-client"
    {
        return sample::run_ui_session_with_open_file_dialog(node_path, client_path);
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
    if arguments.as_slice() == ["--ui-lab"] {
        return win32::run_ui_lab().map_err(Into::into);
    }
    if let [command, document_path] = arguments.as_slice()
        && command == "--ui-preview"
    {
        return run_ui_preview(document_path);
    }
    if !arguments.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: anodrel-windows-host [--ui-lab | --ui-preview <document.json> | --window-lab | --showcase <anodrel.application.json> | --application <anodrel.application.json> | --sample-client <node.exe> <native-client.js> | --sample-ui-client <node.exe> <native-client.js> | --sample-ui-file-client <node.exe> <native-client.js> | --sample-ui-file-text-client <node.exe> <native-client.js> | --sample-ui-save-client <node.exe> <native-client.js> | --sample-ui-storage-client <node.exe> <native-client.js> | --sample-ui-scroll-client <node.exe> <native-client.js> | --sample-ui-diagnostics-client <node.exe> <native-client.js>]",
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
    check_core_health()?;
    run_health_self_test(HostPolicy::new(
        package.identity().application_id(),
        vec![Capability::DiagnosticsRead],
        "anodrel-windows-host",
    )?)?;
    win32::run_startup_lab(package_facts(&package), &instance, started.elapsed())?;
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
mod tests {
    use anodrel_application::ApplicationPackage;

    use super::{
        MAX_ENCODED_DOCUMENT_BYTES, check_core_health, health_display, load_ui_preview_document,
        package_facts, read_bounded_regular_utf8,
    };

    #[test]
    fn displays_a_valid_health_response() {
        let display = health_display(
            r#"{"status":"success","result":{"hostName":"test-host","protocolVersion":{"major":1,"minor":0}}}"#,
        )
        .expect("response is valid");
        assert!(display.contains("status: success"));
        assert!(display.contains("protocol: 1.0"));
    }

    #[test]
    fn startup_lab_requires_a_successful_core_check() {
        let display = check_core_health().expect("core health check is valid");
        assert!(display.contains("status: success"));
    }

    #[test]
    fn displays_only_host_verified_application_metadata() {
        let manifest = r#"{
            "manifestVersion":{"major":1,"minor":0},
            "applicationId":"org.anodrel.sample",
            "displayName":"Anodrel Sample",
            "content":{"format":"anodrel.text.v1","path":"content/main.txt","sha256":"7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"}
        }"#;
        let root =
            std::env::temp_dir().join(format!("anodrel-host-display-test-{}", std::process::id()));
        let content = root.join("content").join("main.txt");
        std::fs::create_dir_all(content.parent().expect("content has parent"))
            .expect("fixture directory is created");
        std::fs::write(root.join("anodrel.application.json"), manifest)
            .expect("fixture manifest is written");
        std::fs::write(&content, "Verified package text.").expect("fixture content is written");

        let package = ApplicationPackage::load(root.join("anodrel.application.json"))
            .expect("fixture package is valid");
        let facts = package_facts(&package);

        assert_eq!(facts.application_id, "org.anodrel.sample");
        assert_eq!(facts.display_name, "Anodrel Sample");
        assert_eq!(facts.content_format, "anodrel.text.v1");
        assert_eq!(facts.content_path, "content/main.txt");
        assert_eq!(facts.content_bytes, "Verified package text.".len());
        // The facts handed to the window layer carry the verified digest and
        // the declared relative path, never a resolved filesystem location.
        assert_eq!(
            facts.content_digest,
            "7089521dabfd335eacdddd28f07cef005bfa68f4aace58c81643e43b6db20585"
        );
        assert!(!facts.content_path.contains(':'));
        std::fs::remove_dir_all(root).expect("fixture directory is removed");
    }

    #[test]
    fn preview_loader_reads_one_bounded_valid_document_before_window_creation() {
        let root =
            std::env::temp_dir().join(format!("anodrel-ui-preview-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture directory is created");
        let document_path = root.join("preview.json");
        std::fs::write(
            &document_path,
            r#"{"format":"anodrel.ui.document.v1","root":{"id":"root","kind":"text","value":"Preview","fontSize":16,"tone":"primary"}}"#,
        )
        .expect("fixture document is written");

        let document = load_ui_preview_document(&document_path).expect("preview is valid");
        assert_eq!(document.root().id().as_str(), "root");

        let oversized_path = root.join("oversized.json");
        std::fs::write(&oversized_path, "x".repeat(MAX_ENCODED_DOCUMENT_BYTES + 1))
            .expect("oversized fixture is written");
        let error = read_bounded_regular_utf8(&oversized_path)
            .expect_err("oversized preview is rejected before decoding");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

        std::fs::remove_dir_all(root).expect("fixture directory is removed");
    }

    #[test]
    fn shipped_ui_preview_document_matches_the_strict_format() {
        let document_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/sample/anodrel.ui.json");
        let document =
            load_ui_preview_document(&document_path).expect("shipped UI preview document is valid");

        assert_eq!(document.root().id().as_str(), "sample.root");
    }
}
