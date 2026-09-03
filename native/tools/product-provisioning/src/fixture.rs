//! The one fixture identity this helper is allowed to provision.
//!
//! Every value here is a compile-time constant. The helper accepts no
//! application ID, display name, executable name, capability list, or content
//! from its command line, so it cannot be pointed at another application's
//! machine-policy key or be used to widen an existing record's grants.

/// The development fixture's application ID, distinct from `org.anodrel.sample`.
pub const APPLICATION_ID: &str = "org.anodrel.product-fixture";

/// The display name the staged manifest carries.
pub const DISPLAY_NAME: &str = "Anodrel Product Fixture";

/// The fixed publisher display text retained only in fixture machine policy.
pub const PUBLISHER_NAME: &str = "Anodrel";

/// The fixed Windows-safe filename retained only in fixture machine policy.
pub const START_MENU_NAME: &str = "Anodrel Product Fixture";

/// The package-relative executable path the record binds.
pub const EXECUTABLE_PATH: &str = "bin/anodrel-product-fixture.exe";

/// The executable file name the provisioning script must stage.
pub const EXECUTABLE_FILE_NAME: &str = "anodrel-product-fixture.exe";

/// The package-relative Anodrel host executable that launches the child.
pub const LAUNCHER_PATH: &str = "bin/anodrel-windows-host.exe";

/// The launcher file name the provisioning script must stage and sign.
pub const LAUNCHER_FILE_NAME: &str = "anodrel-windows-host.exe";

/// The reserved HTTPS host required by record v1.23's update metadata.
///
/// The fixture never performs an update request. This is fixed private record
/// data so its strict development record can exercise the latest parser.
pub const UPDATE_CATALOGUE_HOST: &str = "updates.example.test";

/// The fixed HTTPS port for the fixture's reserved update location.
pub const UPDATE_CATALOGUE_PORT: u16 = 443;

/// The fixed attached-CMS path for the fixture's reserved update location.
pub const UPDATE_CATALOGUE_PATH: &str = "/anodrel/development-fixture.p7s";

/// The package-relative content path the staged manifest declares.
pub const CONTENT_PATH: &str = "content/main.txt";

/// The bounded `anodrel.text.v1` content staged beside the manifest.
///
/// The package text surface is not part of a product session; this exists only
/// so the record parser's package-identity check has a valid package to read.
pub const CONTENT_TEXT: &str = "Anodrel development product fixture.\n\nThis package exists only to give the verified Windows product session a valid machine-policy identity. It is not a product, an installer, or an SDK sample.\n";

/// The exact machine-selected grants the fixture record carries.
///
/// This is the smallest set that can prove a native window round trip. No
/// clipboard, link, dialog, file, storage, credential, or diagnostics grant is
/// requested, so a fixture defect cannot reach those services.
pub const CAPABILITIES: [&str; 3] = ["ui.document.write", "ui.events.read", "session.close"];

#[cfg(test)]
mod tests {
    use anodrel_application::is_valid_application_id;

    use super::{
        APPLICATION_ID, CAPABILITIES, CONTENT_PATH, EXECUTABLE_PATH, LAUNCHER_PATH, START_MENU_NAME,
    };

    #[test]
    fn the_fixture_identity_uses_the_stable_application_grammar() {
        assert!(is_valid_application_id(APPLICATION_ID));
    }

    #[test]
    fn the_fixture_identity_is_not_the_shipped_sample_identity() {
        // Provisioning must never be able to redirect the existing package or
        // Startup Lab identity to a fixture executable.
        assert_ne!(APPLICATION_ID, "org.anodrel.sample");
    }

    #[test]
    fn package_relative_paths_stay_inside_their_package() {
        for path in [EXECUTABLE_PATH, LAUNCHER_PATH, CONTENT_PATH] {
            assert!(!path.contains('\\'));
            assert!(!path.contains(':'));
            assert!(
                path.split('/').all(|component| !component.is_empty()
                    && component != "."
                    && component != "..")
            );
        }
    }

    #[test]
    fn the_fixture_requests_only_its_three_window_grants() {
        assert_eq!(
            CAPABILITIES,
            ["ui.document.write", "ui.events.read", "session.close"]
        );
    }

    #[test]
    fn the_launcher_is_distinct_and_the_start_menu_name_stays_fixed() {
        assert_ne!(LAUNCHER_PATH, EXECUTABLE_PATH);
        assert_eq!(START_MENU_NAME, "Anodrel Product Fixture");
    }
}
