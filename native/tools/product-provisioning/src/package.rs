//! Staging for the fixture's package directory.
//!
//! The staged package is ordinary application input and carries no authority.
//! It exists so the record parser has a valid `anodrel.application.json` to bind
//! its identity to. The record itself always lives outside this directory.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anodrel_application::sha256;

use crate::fixture;

const MANIFEST_NAME: &str = "anodrel.application.json";

/// Creates the fixture package layout and writes its manifest and content.
///
/// The manifest's content digest is computed here from the exact bytes written,
/// so a staged package can never carry a stale digest. The executable is staged
/// separately by the provisioning script, which also signs it.
pub fn stage(package_root: &Path) -> io::Result<()> {
    let content_path = package_root.join("content");
    let executable_directory = package_root.join("bin");
    fs::create_dir_all(&content_path)?;
    fs::create_dir_all(&executable_directory)?;

    let content = fixture::CONTENT_TEXT.as_bytes();
    write_exact(&content_path.join("main.txt"), content)?;
    write_exact(
        &package_root.join(MANIFEST_NAME),
        manifest(content).as_bytes(),
    )
}

/// Returns the canonical child path the record will bind, if it exists.
pub fn executable(package_root: &Path) -> io::Result<PathBuf> {
    staged_image(package_root, fixture::EXECUTABLE_FILE_NAME)
}

/// Returns the canonical host-launcher path the record will bind, if it exists.
pub fn launcher(package_root: &Path) -> io::Result<PathBuf> {
    staged_image(package_root, fixture::LAUNCHER_FILE_NAME)
}

fn staged_image(package_root: &Path, file_name: &str) -> io::Result<PathBuf> {
    let path = fs::canonicalize(package_root.join("bin").join(file_name))?;
    if fs::metadata(&path)?.is_file() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the staged fixture image is not a regular file",
        ))
    }
}

/// Builds the strict manifest for exactly the bytes staged as content.
fn manifest(content: &[u8]) -> String {
    let digest = sha256::to_lower_hex(&sha256::digest(content));
    format!(
        concat!(
            "{{\n",
            "  \"manifestVersion\": {{ \"major\": 1, \"minor\": 0 }},\n",
            "  \"applicationId\": \"{}\",\n",
            "  \"displayName\": \"{}\",\n",
            "  \"content\": {{\n",
            "    \"format\": \"anodrel.text.v1\",\n",
            "    \"path\": \"{}\",\n",
            "    \"sha256\": \"{}\"\n",
            "  }}\n",
            "}}\n"
        ),
        fixture::APPLICATION_ID,
        fixture::DISPLAY_NAME,
        fixture::CONTENT_PATH,
        digest
    )
}

fn write_exact(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use anodrel_application::ApplicationManifest;

    use super::{fixture, launcher, manifest, stage};

    #[test]
    fn the_staged_manifest_parses_and_matches_its_own_content_digest() {
        let parsed = ApplicationManifest::parse(&manifest(fixture::CONTENT_TEXT.as_bytes()))
            .expect("the staged manifest is valid");

        assert_eq!(parsed.identity().application_id(), fixture::APPLICATION_ID);
        assert_eq!(parsed.identity().display_name(), fixture::DISPLAY_NAME);
        assert_eq!(parsed.content_path(), fixture::CONTENT_PATH);
    }

    #[test]
    fn staging_writes_a_package_the_validator_accepts() {
        let root =
            std::env::temp_dir().join(format!("anodrel-fixture-stage-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        stage(&root).expect("the fixture package stages");

        let package =
            anodrel_application::ApplicationPackage::load(root.join("anodrel.application.json"))
                .expect("the staged package validates");
        assert_eq!(package.identity().application_id(), fixture::APPLICATION_ID);
        // The executable directory exists but is deliberately still empty: the
        // provisioning script stages and signs both images separately.
        assert!(root.join("bin").is_dir());
        assert!(launcher(&root).is_err());

        std::fs::remove_dir_all(root).expect("the fixture staging directory is removed");
    }
}
