//! Creation and isolated-build verification for generated native projects.

use std::{error::Error, fmt, fs, path::Path};

use crate::{
    paths::{anodrel_root, relative_path, resolve_new_project, write_new_file},
    template::{TemplateContext, cargo_toml, main_source, readme},
    validation::{validate_display_label, validate_project_slug},
};

#[derive(Debug)]
pub struct InitError(&'static str);

impl InitError {
    pub const fn new(message: &'static str) -> Self {
        Self(message)
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for InitError {}

pub fn initialize(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    validate_project_slug(project_slug)?;
    validate_display_label(display_label)?;
    let project_directory = resolve_new_project(destination)?;
    let root = anodrel_root()?;
    let context = template_context(&project_directory, &root, project_slug)?;
    let manifest = cargo_toml(&context);
    let source = main_source(display_label);
    let project_readme = readme(&context);

    fs::create_dir(&project_directory)
        .map_err(|_| InitError::new("could not create project directory"))?;
    let source_directory = project_directory.join("src");
    fs::create_dir(&source_directory)
        .map_err(|_| InitError::new("could not create project source directory"))?;
    write_new_file(&project_directory.join("Cargo.toml"), &manifest)?;
    write_new_file(&project_directory.join("README.md"), &project_readme)?;
    write_new_file(&source_directory.join("main.rs"), &source)?;

    println!("Created Anodrel native UI project.");
    Ok(())
}

fn template_context(
    project_directory: &Path,
    root: &Path,
    project_slug: &str,
) -> Result<TemplateContext, InitError> {
    Ok(TemplateContext {
        project_slug: project_slug.to_owned(),
        client_path: relative_path(project_directory, &root.join("native/crates/client"))?,
        ui_client_path: relative_path(project_directory, &root.join("native/crates/ui-client"))?,
        windows_client_path: relative_path(
            project_directory,
            &root.join("native/adapters/windows-client"),
        )?,
        host_manifest_path: relative_path(project_directory, &root.join("native/Cargo.toml"))?,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        process::Command,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::initialize;

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory {
        path: std::path::PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let temporary = std::env::temp_dir();
            let path = temporary.join(format!(
                "anodrel-native-app-tool-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create unique native-app-tool test directory");
            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let temporary = std::env::temp_dir();
            let expected_prefix = format!("anodrel-native-app-tool-test-{}-", std::process::id());
            let name = self.path.file_name().and_then(|name| name.to_str());
            if self.path.parent() == Some(temporary.as_path())
                && name.is_some_and(|name| name.starts_with(&expected_prefix))
            {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }

    #[test]
    fn refuses_an_existing_destination_without_writing_into_it() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("existing-app");
        fs::create_dir(&destination).expect("create existing destination");
        assert!(initialize(&destination, "existing-app", "Existing App").is_err());
        assert!(!destination.join("Cargo.toml").exists());
    }

    #[test]
    fn generated_project_builds_in_isolation_with_relative_first_party_paths() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-app");
        initialize(
            &destination,
            "generated-app",
            "Generated \"Template\" \\ App",
        )
        .expect("generate a new project");

        let manifest =
            fs::read_to_string(destination.join("Cargo.toml")).expect("read generated manifest");
        let root = crate::paths::anodrel_root().expect("locate checkout");
        assert!(!manifest.contains(&root.to_string_lossy().to_string()));
        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated project instructions");
        assert!(readme.contains("--native-template-client"));
        assert!(destination.join("src/main.rs").is_file());

        let status = Command::new(env!("CARGO"))
            .arg("build")
            .arg("--quiet")
            .arg("--release")
            .arg("--manifest-path")
            .arg(destination.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", temporary.path.join("target"))
            .status()
            .expect("run cargo check for generated project");
        assert!(status.success(), "generated project must compile");
    }
}
