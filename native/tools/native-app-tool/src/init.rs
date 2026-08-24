//! Creation and isolated-build verification for generated native projects.

use std::{error::Error, fmt, fs, path::Path};

use crate::{
    arguments::TemplateKind,
    paths::{anodrel_root, relative_path, resolve_new_project, write_new_file},
    template::{
        TemplateContext, cargo_toml, form_main_source, form_readme, live_status_main_source,
        live_status_readme, main_source, menu_main_source, menu_readme, multi_window_main_source,
        multi_window_readme, readme, scroll_window_main_source, scroll_window_readme,
    },
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
    initialize_template(TemplateKind::Ui, destination, project_slug, display_label)
}

pub fn initialize_form(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(TemplateKind::Form, destination, project_slug, display_label)
}

pub fn initialize_live_status(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(
        TemplateKind::LiveStatus,
        destination,
        project_slug,
        display_label,
    )
}

pub fn initialize_menu(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(TemplateKind::Menu, destination, project_slug, display_label)
}

pub fn initialize_multi_window(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(
        TemplateKind::MultiWindow,
        destination,
        project_slug,
        display_label,
    )
}

pub fn initialize_scroll_window(
    destination: &Path,
    project_slug: &str,
    display_label: &str,
) -> Result<(), InitError> {
    initialize_template(
        TemplateKind::ScrollWindow,
        destination,
        project_slug,
        display_label,
    )
}

fn initialize_template(
    template_kind: TemplateKind,
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
    let (source, project_readme, created_message) = match template_kind {
        TemplateKind::Ui => (
            main_source(display_label),
            readme(&context),
            "Created Anodrel native UI project.",
        ),
        TemplateKind::Form => (
            form_main_source(display_label),
            form_readme(&context),
            "Created Anodrel native form project.",
        ),
        TemplateKind::LiveStatus => (
            live_status_main_source(display_label),
            live_status_readme(&context),
            "Created Anodrel native live-status project.",
        ),
        TemplateKind::Menu => (
            menu_main_source(display_label),
            menu_readme(&context),
            "Created Anodrel native menu project.",
        ),
        TemplateKind::MultiWindow => (
            multi_window_main_source(display_label),
            multi_window_readme(&context),
            "Created Anodrel native multi-window project.",
        ),
        TemplateKind::ScrollWindow => (
            scroll_window_main_source(display_label),
            scroll_window_readme(&context),
            "Created Anodrel native scroll-window project.",
        ),
    };

    fs::create_dir(&project_directory)
        .map_err(|_| InitError::new("could not create project directory"))?;
    let source_directory = project_directory.join("src");
    fs::create_dir(&source_directory)
        .map_err(|_| InitError::new("could not create project source directory"))?;
    write_new_file(&project_directory.join("Cargo.toml"), &manifest)?;
    write_new_file(&project_directory.join("README.md"), &project_readme)?;
    write_new_file(&source_directory.join("main.rs"), &source)?;

    println!("{created_message}");
    Ok(())
}

fn template_context(
    project_directory: &Path,
    root: &Path,
    project_slug: &str,
) -> Result<TemplateContext, InitError> {
    Ok(TemplateContext {
        project_slug: project_slug.to_owned(),
        windows_ui_sdk_path: relative_path(
            project_directory,
            &root.join("native/adapters/windows-ui-sdk"),
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

    use super::{
        initialize, initialize_form, initialize_live_status, initialize_menu,
        initialize_multi_window, initialize_scroll_window,
    };

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

    #[test]
    fn menu_project_is_separate_from_the_regular_template() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-menu-app");
        initialize_menu(&destination, "generated-menu-app", "Generated Menu App")
            .expect("generate a native menu project");

        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated menu instructions");
        let source = fs::read_to_string(destination.join("src/main.rs"))
            .expect("read generated menu source");
        assert!(readme.contains("--native-menu-template-client"));
        assert!(source.contains("replace_menu_v1"));
        assert!(source.contains("template.menu.complete"));
        assert!(!source.contains("template.complete"));
    }

    #[test]
    fn form_project_is_separate_from_the_other_templates() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-form-app");
        initialize_form(&destination, "generated-form-app", "Generated Form App")
            .expect("generate a native form project");

        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated form instructions");
        let source = fs::read_to_string(destination.join("src/main.rs"))
            .expect("read generated form source");
        assert!(readme.contains("--native-form-template-client"));
        assert!(source.contains("read_fields"));
        assert!(source.contains("template.form.name"));
        assert!(source.contains("template.form.submit"));
        assert!(!source.contains("replace_menu_v1"));
    }

    #[test]
    fn live_status_project_uses_only_explicit_v3_replacements() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-live-status-app");
        initialize_live_status(
            &destination,
            "generated-live-status-app",
            "Generated Live Status App",
        )
        .expect("generate a native live-status project");

        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated live-status instructions");
        let source = fs::read_to_string(destination.join("src/main.rs"))
            .expect("read generated live-status source");
        assert!(readme.contains("--native-live-status-template-client"));
        assert_eq!(source.matches("replace_document_v3(").count(), 3);
        assert!(source.contains("template.status.polite"));
        assert!(source.contains("template.status.assertive"));
        assert!(source.contains("template.status.complete"));
        assert!(!source.contains("replace_document_v1("));
    }

    #[test]
    fn multi_window_project_is_separate_from_the_other_templates() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-multi-window-app");
        initialize_multi_window(
            &destination,
            "generated-multi-window-app",
            "Generated Multi-Window App",
        )
        .expect("generate a native multi-window project");

        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated multi-window instructions");
        let source = fs::read_to_string(destination.join("src/main.rs"))
            .expect("read generated multi-window source");
        assert!(readme.contains("--native-multi-window-template-client"));
        assert!(source.contains("open_window_v1"));
        assert!(source.contains("replace_window_document_v1"));
        assert!(source.contains("read_window_actions"));
        assert!(source.contains("close_window"));
        assert!(!source.contains("replace_menu_v1"));
    }

    #[test]
    fn scroll_window_project_is_separate_from_the_other_templates() {
        let temporary = TestDirectory::new();
        let destination = temporary.path.join("generated-scroll-window-app");
        initialize_scroll_window(
            &destination,
            "generated-scroll-window-app",
            "Generated Scroll Window App",
        )
        .expect("generate a native scroll-window project");

        let readme = fs::read_to_string(destination.join("README.md"))
            .expect("read generated scroll-window instructions");
        let source = fs::read_to_string(destination.join("src/main.rs"))
            .expect("read generated scroll-window source");
        assert!(readme.contains("--native-scroll-window-template-client"));
        assert!(source.contains("open_window_v2"));
        assert!(source.contains("replace_window_document_v2"));
        assert!(source.contains("read_window_actions"));
        assert!(source.contains("close_window"));
        assert!(!source.contains("open_window_v1"));
        assert!(!source.contains("replace_window_document_v1"));
    }
}
