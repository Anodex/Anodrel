//! Package-tool execution over the owned application-package module.

use std::{error::Error, fmt};

use anodrel_application::{ApplicationError, ApplicationPackage, write_text_package};

use crate::arguments::Command;

#[derive(Debug)]
pub struct ToolError(&'static str);

impl fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for ToolError {}

pub fn run(command: Command) -> Result<(), ToolError> {
    match command {
        Command::Init {
            destination,
            application_id,
            display_name,
            content,
        } => init(&destination, &application_id, &display_name, &content),
        Command::Verify { manifest_path } => verify(&manifest_path),
    }
}

fn init(
    destination: &str,
    application_id: &str,
    display_name: &str,
    content: &str,
) -> Result<(), ToolError> {
    let normalised = content.replace("\r\n", "\n").replace('\r', "\n");
    let manifest = write_text_package(destination, application_id, display_name, &normalised)
        .map_err(package_error)?;
    let package = ApplicationPackage::load(&manifest).map_err(package_error)?;

    println!(
        "Created Anodrel package: {}",
        manifest.parent().unwrap_or(&manifest).display()
    );
    print_package_facts(&package);
    Ok(())
}

fn verify(manifest_path: &str) -> Result<(), ToolError> {
    let package = ApplicationPackage::load(manifest_path).map_err(package_error)?;
    print_package_facts(&package);
    Ok(())
}

fn print_package_facts(package: &ApplicationPackage) {
    println!("applicationId={}", package.identity().application_id());
    println!("displayName={}", package.identity().display_name());
    println!("contentPath={}", package.content().path());
    println!("sha256={}", package.content().digest());
    println!("contentBytes={}", package.content().byte_length());
}

fn package_error(error: ApplicationError) -> ToolError {
    let message = match error {
        ApplicationError::PackageDestinationExists => "package destination already exists",
        ApplicationError::InvalidPackageDestination => "package destination is invalid",
        ApplicationError::Io(_) => "could not read or write the application package",
        ApplicationError::ManifestTooLarge => "application manifest exceeds its limit",
        ApplicationError::ContentTooLarge => "application content exceeds its limit",
        ApplicationError::InvalidManifest => "application manifest identity or format is invalid",
        ApplicationError::InvalidContentPath => "application content path is invalid",
        ApplicationError::ContentOutsidePackage => {
            "application content resolves outside its package"
        }
        ApplicationError::ContentDigestMismatch => "application content digest does not match",
        ApplicationError::InvalidText => "application text is invalid",
    };
    ToolError(message)
}
