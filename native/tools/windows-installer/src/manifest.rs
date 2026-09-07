//! Strict `anodrel.release.v1` manifest facts.

use anodrel_application::{StartMenuName, UpdateCatalogueLocation};
use anodrel_network::NetworkOrigin;
use anodrel_protocol::Capability;

pub use anodrel_application::ProductDisplayMetadata as ProductMetadata;

mod fields;
mod format;
mod launcher;
mod parser;
mod product;
mod version;

pub use launcher::ProductLauncher;
pub use version::PackageVersion;

/// The bounded embedded-payload facts a signed manifest declares.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PayloadDescriptor {
    byte_length: u64,
    digest: [u8; 32],
}

impl PayloadDescriptor {
    /// Returns the exact uncompressed payload byte length.
    #[must_use]
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }

    /// Compares a calculated payload digest without exposing it as display text.
    #[must_use]
    pub fn matches_digest(self, actual: [u8; 32]) -> bool {
        self.digest == actual
    }
}

impl std::fmt::Debug for PayloadDescriptor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PayloadDescriptor")
            .field("byte_length", &self.byte_length)
            .field("digest", &"[redacted]")
            .finish()
    }
}

/// One exact application release selected by a signed installer image.
pub struct ReleaseManifest {
    application_id: String,
    package_version: PackageVersion,
    executable_path: String,
    executable_digest: [u8; 32],
    publisher_fingerprint: [u8; 32],
    capabilities: Vec<Capability>,
    network_origins: Vec<NetworkOrigin>,
    update_catalogue: Option<UpdateCatalogueLocation>,
    product_metadata: Option<ProductMetadata>,
    start_menu_name: Option<StartMenuName>,
    product_launcher: Option<ProductLauncher>,
    payload: PayloadDescriptor,
}

impl ReleaseManifest {
    /// Returns the signed application identity selected for this release.
    #[must_use]
    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    /// Returns the signed version used for the owned release directory.
    #[must_use]
    pub const fn package_version(&self) -> PackageVersion {
        self.package_version
    }

    /// Returns the relative contained executable path.
    #[must_use]
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    /// Compares a calculated executable digest without exposing it as text.
    #[must_use]
    pub fn matches_executable_digest(&self, actual: [u8; 32]) -> bool {
        self.executable_digest == actual
    }

    pub(super) const fn executable_digest(&self) -> &[u8; 32] {
        &self.executable_digest
    }

    /// Compares a Windows Authenticode leaf fingerprint without exposing it.
    #[must_use]
    pub fn matches_publisher_fingerprint(&self, actual: [u8; 32]) -> bool {
        self.publisher_fingerprint == actual
    }

    pub(super) const fn publisher_fingerprint(&self) -> &[u8; 32] {
        &self.publisher_fingerprint
    }

    /// Returns the machine-selected capability set embedded in this release.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Returns the exact embedded network origins.
    #[must_use]
    pub fn network_origins(&self) -> &[NetworkOrigin] {
        &self.network_origins
    }

    /// Returns the signed update-catalogue source, when this release opted in.
    #[must_use]
    pub fn update_catalogue_location(&self) -> Option<&UpdateCatalogueLocation> {
        self.update_catalogue.as_ref()
    }

    /// Returns signed display metadata when this release declares version 1.2.
    #[must_use]
    pub fn product_metadata(&self) -> Option<&ProductMetadata> {
        self.product_metadata.as_ref()
    }

    /// Returns the signed Windows-safe Start-menu filename when declared.
    #[must_use]
    pub fn start_menu_name(&self) -> Option<&StartMenuName> {
        self.start_menu_name.as_ref()
    }

    /// Returns the signed product launcher when this release declares version 1.4.
    #[must_use]
    pub fn product_launcher(&self) -> Option<&ProductLauncher> {
        self.product_launcher.as_ref()
    }

    /// Returns the signed embedded-payload descriptor.
    #[must_use]
    pub const fn payload(&self) -> PayloadDescriptor {
        self.payload
    }
}

impl std::fmt::Debug for ReleaseManifest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReleaseManifest")
            .field("application_id", &self.application_id)
            .field("package_version", &self.package_version)
            .field("executable_path", &self.executable_path)
            .field("executable_digest", &"[redacted]")
            .field("publisher_fingerprint", &"[redacted]")
            .field("capabilities", &self.capabilities)
            .field("network_origins", &self.network_origins)
            .field("update_catalogue", &self.update_catalogue)
            .field("product_metadata", &self.product_metadata)
            .field("start_menu_name", &self.start_menu_name)
            .field("product_launcher", &self.product_launcher)
            .field("payload", &self.payload)
            .finish()
    }
}
