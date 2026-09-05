//! Startup checks and manifest-backed host surface preparation.
//!
//! Command parsing stays in `command`; this module owns the bounded file,
//! package, single-instance, and protocol work performed before a host surface
//! can appear. It intentionally owns no native command-line parsing.

use std::{
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

use crate::{product, win32};

/// Loads one bounded regular-file UI document for the explicit developer preview.
///
/// This is intentionally not an application, package, or session loader. It
/// reads only the named local file, then validates the whole document before
/// the caller creates a native window.
pub(crate) fn load_ui_preview_document(path: &Path) -> Result<UiDocument, Box<dyn Error>> {
    let encoded = read_bounded_regular_utf8(path)?;
    Ok(decode(&encoded)?)
}

/// Opens one validated UI-preview document through the existing host surface.
pub(crate) fn run_ui_preview(document_path: &str) -> Result<(), Box<dyn Error>> {
    let document = load_ui_preview_document(Path::new(document_path))?;
    win32::run_ui_preview(document)?;
    Ok(())
}

/// Reads one bounded regular UTF-8 preview source before decoding it.
pub(crate) fn read_bounded_regular_utf8(path: &Path) -> io::Result<String> {
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

/// Opens the ordinary host diagnostic after its closed core health check.
pub(crate) fn run_diagnostics_window() -> Result<(), Box<dyn Error>> {
    let display = check_core_health()?;
    win32::run("Anodrel Windows host", &display)?;
    Ok(())
}

/// Opens the single-instance Startup Lab after its complete preflight.
pub(crate) fn run_startup_lab(manifest_path: &str, started: Instant) -> Result<(), Box<dyn Error>> {
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
pub(crate) fn run_startup_report(
    manifest_path: &str,
    started: Instant,
) -> Result<(), Box<dyn Error>> {
    let package = ApplicationPackage::load(manifest_path)?;
    let _ = complete_startup_checks(&package)?;
    win32::print_startup_report(package.identity().application_id(), started.elapsed());
    Ok(())
}

/// Copies the display-safe facts out of a validated package.
///
/// The window layer never receives the package itself, so it cannot reach a
/// resolved filesystem path or any value that skipped validation.
pub(crate) fn package_facts(package: &ApplicationPackage) -> win32::PackageFacts {
    win32::PackageFacts {
        display_name: package.identity().display_name().to_owned(),
        application_id: package.identity().application_id().to_owned(),
        content_format: package.content().format().to_owned(),
        content_path: package.content().path().to_owned(),
        content_digest: package.content().digest().to_owned(),
        content_bytes: package.content().byte_length(),
    }
}

/// Runs the closed host-health request used by ordinary diagnostics and preflight.
pub(crate) fn check_core_health() -> Result<String, Box<dyn Error>> {
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

/// Opens one digest-verified package through the ordinary single-instance route.
pub(crate) fn run_application_window(manifest_path: &str) -> Result<(), Box<dyn Error>> {
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

/// Converts the closed core health response into a host-owned diagnostic display.
pub(crate) fn health_display(response: &str) -> Result<String, Box<dyn Error>> {
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
