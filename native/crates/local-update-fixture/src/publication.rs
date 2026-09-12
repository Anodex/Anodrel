//! Fixed artifact validation for one local update-fixture publication root.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// The only identity allowed in the local update acceptance fixture.
pub const APPLICATION_ID: &str = "org.anodrel.local-update-fixture";
/// The installed release version for the local update acceptance fixture.
pub const INITIAL_VERSION: &str = "0.1.0";
/// The only newer release version published by the fixture.
pub const UPDATE_VERSION: &str = "0.1.1";
/// The fixed local HTTPS hostname.
pub const LOCALHOST: &str = "localhost";
/// The fixed local HTTPS port.
pub const PORT: u16 = 45_863;
/// The only catalogue target accepted by the local fixture server.
pub const CATALOGUE_REQUEST_TARGET: &str = "/anodrel/local-update/stable.p7s";
/// The only installer target accepted by the local fixture server.
pub const INSTALLER_REQUEST_TARGET: &str = "/anodrel/local-update/releases/0.1.1/installer.exe";

const CATALOGUE_RELATIVE_PATH: &str = "stable.p7s";
const INSTALLER_RELATIVE_PATH: &str = "releases/0.1.1/installer.exe";
const MAXIMUM_CATALOGUE_BYTES: u64 = 128 * 1024;
const MAXIMUM_INSTALLER_BYTES: u64 = 576 * 1024 * 1024;

/// The two regular checked files a fixture server may publish.
#[derive(Debug)]
pub struct FixturePublication {
    root: PathBuf,
    catalogue: Artifact,
    installer: Artifact,
}

impl FixturePublication {
    /// Opens one fixed publication root after checking its two allowed files.
    ///
    /// No request, command-line, environment, or caller-provided relative path
    /// selects an artifact. Both files must be ordinary nonempty files under
    /// `root` and within the same fixed bounds as the update client.
    pub fn open(root: &Path) -> Result<Self, FixturePublicationError> {
        let root = canonical_directory(root)?;
        let catalogue = checked_artifact(&root, CATALOGUE_RELATIVE_PATH, MAXIMUM_CATALOGUE_BYTES)?;
        let installer = checked_artifact(&root, INSTALLER_RELATIVE_PATH, MAXIMUM_INSTALLER_BYTES)?;
        Ok(Self {
            root,
            catalogue,
            installer,
        })
    }

    /// Returns the fixed canonical publication directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the checked CMS catalogue path and byte length.
    #[must_use]
    pub fn catalogue(&self) -> (&Path, u64) {
        self.catalogue.parts()
    }

    /// Returns the checked candidate-installer path and byte length.
    #[must_use]
    pub fn installer(&self) -> (&Path, u64) {
        self.installer.parts()
    }
}

/// A closed reason why a fixture publication cannot be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixturePublicationError {
    /// The publication root was missing, non-canonical, linked, or not a directory.
    RootInvalid,
    /// A fixed publication artifact was missing, linked, outside its root, or not regular.
    ArtifactInvalid,
    /// A fixed publication artifact was empty or exceeded its fixed byte bound.
    ArtifactSizeInvalid,
}

impl std::fmt::Display for FixturePublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::RootInvalid => "the fixed local update publication root is invalid",
            Self::ArtifactInvalid => "a fixed local update publication artifact is invalid",
            Self::ArtifactSizeInvalid => {
                "a fixed local update publication artifact has an invalid size"
            }
        })
    }
}

impl std::error::Error for FixturePublicationError {}

#[derive(Debug)]
struct Artifact {
    path: PathBuf,
    bytes: u64,
}

impl Artifact {
    fn parts(&self) -> (&Path, u64) {
        (&self.path, self.bytes)
    }
}

fn canonical_directory(root: &Path) -> Result<PathBuf, FixturePublicationError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| FixturePublicationError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FixturePublicationError::RootInvalid);
    }
    fs::canonicalize(root).map_err(|_| FixturePublicationError::RootInvalid)
}

fn checked_artifact(
    root: &Path,
    relative: &str,
    maximum_bytes: u64,
) -> Result<Artifact, FixturePublicationError> {
    let candidate = root.join(relative);
    let metadata =
        fs::symlink_metadata(&candidate).map_err(|_| FixturePublicationError::ArtifactInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(FixturePublicationError::ArtifactInvalid);
    }
    let path =
        fs::canonicalize(&candidate).map_err(|_| FixturePublicationError::ArtifactInvalid)?;
    if !path.starts_with(root) {
        return Err(FixturePublicationError::ArtifactInvalid);
    }
    let bytes = metadata.len();
    if bytes == 0 || bytes > maximum_bytes {
        return Err(FixturePublicationError::ArtifactSizeInvalid);
    }
    Ok(Artifact { path, bytes })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{
        APPLICATION_ID, CATALOGUE_REQUEST_TARGET, FixturePublication, FixturePublicationError,
        INITIAL_VERSION, INSTALLER_REQUEST_TARGET, LOCALHOST, PORT, UPDATE_VERSION,
    };

    #[test]
    fn publication_accepts_only_the_fixed_two_artifacts() {
        let fixture = TestDirectory::new();
        fixture.write("stable.p7s", b"cms");
        fixture.write("releases/0.1.1/installer.exe", b"signed image");

        let publication = FixturePublication::open(fixture.path()).unwrap();
        assert_eq!(
            publication.root(),
            fs::canonicalize(fixture.path()).unwrap()
        );
        assert_eq!(publication.catalogue().1, 3);
        assert_eq!(publication.installer().1, 12);
    }

    #[test]
    fn missing_or_empty_artifacts_fail_closed() {
        let fixture = TestDirectory::new();
        fixture.write("stable.p7s", b"");
        fixture.write("releases/0.1.1/installer.exe", b"signed image");
        assert!(matches!(
            FixturePublication::open(fixture.path()),
            Err(FixturePublicationError::ArtifactSizeInvalid)
        ));
        fixture.write("stable.p7s", b"cms");
        fs::remove_file(fixture.path().join("releases/0.1.1/installer.exe")).unwrap();
        assert!(matches!(
            FixturePublication::open(fixture.path()),
            Err(FixturePublicationError::ArtifactInvalid)
        ));
    }

    #[test]
    fn fixture_facts_leave_no_dynamic_identity_or_endpoint() {
        assert_eq!(APPLICATION_ID, "org.anodrel.local-update-fixture");
        assert_eq!(INITIAL_VERSION, "0.1.0");
        assert_eq!(UPDATE_VERSION, "0.1.1");
        assert_eq!(LOCALHOST, "localhost");
        assert_eq!(PORT, 45_863);
        assert_eq!(CATALOGUE_REQUEST_TARGET, "/anodrel/local-update/stable.p7s");
        assert_eq!(
            INSTALLER_REQUEST_TARGET,
            "/anodrel/local-update/releases/0.1.1/installer.exe"
        );
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "anodrel-local-update-fixture-test-{}-{}",
                std::process::id(),
                NEXT_TEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            let path = self.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    static NEXT_TEST: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}
