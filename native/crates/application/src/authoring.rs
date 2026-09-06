//! First-party creation of the current verified text-package format.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::{
    ApplicationError, ApplicationPackage, is_valid_application_id, is_valid_display_name, sha256,
    validate_text_content,
};

const CONTENT_PATH: &str = "content/main.txt";
const ATTRIBUTES: &[u8] = b"content/main.txt -text\n";

/// Creates one new strict `anodrel.text.v1` package and returns its manifest.
///
/// The destination itself must not exist. The caller supplies already
/// normalised LF text; this function validates every input before it creates a
/// directory and reloads the result through `ApplicationPackage::load` before
/// returning it.
pub fn write_text_package(
    destination: impl AsRef<Path>,
    application_id: &str,
    display_name: &str,
    text: &str,
) -> Result<PathBuf, ApplicationError> {
    if !is_valid_application_id(application_id) || !is_valid_display_name(display_name) {
        return Err(ApplicationError::InvalidManifest);
    }
    validate_text_content(text)?;

    let destination = destination.as_ref();
    if destination.exists() {
        return Err(ApplicationError::PackageDestinationExists);
    }
    if destination.file_name().is_none() {
        return Err(ApplicationError::InvalidPackageDestination);
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    fs::create_dir_all(parent)?;
    fs::create_dir(destination).map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            ApplicationError::PackageDestinationExists
        } else {
            ApplicationError::Io(error)
        }
    })?;

    let content_path = destination.join(CONTENT_PATH);
    let content_directory = content_path
        .parent()
        .expect("fixed content path has a parent");
    fs::create_dir(content_directory)?;
    let content_bytes = text.as_bytes();
    let digest = sha256::to_lower_hex(&sha256::digest(content_bytes));
    fs::write(content_path, content_bytes)?;
    fs::write(destination.join(".gitattributes"), ATTRIBUTES)?;

    let manifest_path = destination.join("anodrel.application.json");
    fs::write(
        &manifest_path,
        manifest_json(application_id, display_name, &digest),
    )?;
    ApplicationPackage::load(&manifest_path)?;

    Ok(manifest_path)
}

fn manifest_json(application_id: &str, display_name: &str, digest: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"manifestVersion\": {{ \"major\": 1, \"minor\": 0 }},\n",
            "  \"applicationId\": {},\n",
            "  \"displayName\": {},\n",
            "  \"content\": {{\n",
            "    \"format\": \"anodrel.text.v1\",\n",
            "    \"path\": \"content/main.txt\",\n",
            "    \"sha256\": \"{}\"\n",
            "  }}\n",
            "}}\n"
        ),
        json_string(application_id),
        json_string(display_name),
        digest,
    )
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                use std::fmt::Write;
                write!(output, "\\u{:04x}", u32::from(control))
                    .expect("writing to a string succeeds");
            }
            ordinary => output.push(ordinary),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::write_text_package;
    use crate::{ApplicationError, ApplicationPackage, test_support::TestDirectory};

    #[test]
    fn writes_a_package_the_loader_independently_accepts() {
        let root = TestDirectory::new("application-authoring");
        let destination = root.path().join("nested").join("starter");
        let manifest = write_text_package(
            &destination,
            "org.example.starter",
            "Starter \"package\"",
            "First line\nSecond line",
        )
        .expect("package is written");

        let package =
            ApplicationPackage::load(&manifest).expect("loader accepts generated package");
        assert_eq!(package.identity().application_id(), "org.example.starter");
        assert_eq!(package.identity().display_name(), "Starter \"package\"");
        assert_eq!(package.text(), "First line\nSecond line");
        assert_eq!(
            fs::read(destination.join(".gitattributes")).expect("attributes are written"),
            b"content/main.txt -text\n"
        );

        root.remove();
    }

    #[test]
    fn refuses_invalid_inputs_without_creating_the_destination() {
        let root = TestDirectory::new("application-authoring");
        let destination = root.path().join("starter");
        assert!(matches!(
            write_text_package(&destination, "Invalid.Id", "Starter", "Valid"),
            Err(ApplicationError::InvalidManifest)
        ));
        assert!(!destination.exists());

        let invalid_display = root.path().join("invalid-display");
        assert!(matches!(
            write_text_package(
                &invalid_display,
                "org.example.starter",
                "Invalid\tname",
                "Valid"
            ),
            Err(ApplicationError::InvalidManifest)
        ));
        assert!(!invalid_display.exists());

        let invalid_text = root.path().join("invalid-text");
        assert!(matches!(
            write_text_package(
                &invalid_text,
                "org.example.starter",
                "Starter",
                "Invalid\ttext"
            ),
            Err(ApplicationError::InvalidText)
        ));
        assert!(!invalid_text.exists());

        fs::create_dir_all(&destination).expect("existing destination is created");
        assert!(matches!(
            write_text_package(&destination, "org.example.starter", "Starter", "Valid"),
            Err(ApplicationError::PackageDestinationExists)
        ));

        root.remove();
    }
}
