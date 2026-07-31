#![deny(unsafe_op_in_unsafe_fn)]

mod sample;
mod win32;

use std::{env, error::Error, io};

use anodrel_application::ApplicationPackage;
use anodrel_core::{CoreHost, HostPolicy};
use anodrel_protocol::{Capability, JsonValue};
use anodrel_windows_instance::{InstanceClaim, InstanceScope, claim};
use anodrel_windows_pipe::run_health_self_test;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if let [command, node_path, client_path] = arguments.as_slice()
        && command == "--sample-client"
    {
        return sample::run(node_path, client_path);
    }
    if let [command, manifest_path] = arguments.as_slice()
        && command == "--application"
    {
        return run_application_window(manifest_path);
    }
    if let [command, manifest_path] = arguments.as_slice()
        && command == "--showcase"
    {
        return run_startup_lab(manifest_path);
    }
    if arguments.as_slice() == ["--window-lab"] {
        return win32::run_window_lab().map_err(Into::into);
    }
    if !arguments.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: anodrel-windows-host [--window-lab | --showcase <anodrel.application.json> | --application <anodrel.application.json> | --sample-client <node.exe> <native-client.js>]",
        )
        .into());
    }
    run_diagnostics_window()
}

fn run_diagnostics_window() -> Result<(), Box<dyn Error>> {
    let display = check_core_health()?;
    win32::run("Anodrel Windows host", &display)?;
    Ok(())
}

fn run_startup_lab(manifest_path: &str) -> Result<(), Box<dyn Error>> {
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
    win32::run_startup_lab(
        package.identity().display_name(),
        package.identity().application_id(),
        &instance,
    )?;
    Ok(())
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
    let display = application_display(&package);
    win32::run_application(&title, &display, &instance)?;
    Ok(())
}

fn application_display(package: &ApplicationPackage) -> String {
    format!(
        "Anodrel verified application package\n\napplication: {}\ncontent integrity: verified\nformat: anodrel.text.v1\n\n{}",
        package.identity().application_id(),
        package.text()
    )
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
        "Anodrel direct Windows host\n\nThe window, message loop, UTF-16 conversion, drawing, JSON codec, and protocol core are owned by Anodrel.\n\nStartup protocol check\nstatus: {status}\nhost: {host_name}\nprotocol: {major}.{minor}"
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

    use super::{application_display, check_core_health, health_display};

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
    fn startup_lab_requires_a_successful_owned_core_check() {
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
        let display = application_display(&package);

        assert!(display.contains("application: org.anodrel.sample"));
        assert!(display.contains("Verified package text."));
        std::fs::remove_dir_all(root).expect("fixture directory is removed");
    }
}
