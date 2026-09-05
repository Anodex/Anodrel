//! Fixed project plans and writing for each native development template.

use std::{fs, path::Path};

use crate::{
    arguments::TemplateKind,
    paths::{anodrel_root, relative_path, resolve_new_project, write_new_file},
    template::{
        TemplateContext, cargo_toml, context_menu_main_source, context_menu_readme,
        file_binary_write_main_source, file_binary_write_readme, file_write_main_source,
        file_write_readme, form_main_source, form_readme, live_status_main_source,
        live_status_readme, main_source, menu_main_source, menu_readme, multi_window_main_source,
        multi_window_readme, notification_main_source, notification_readme, readme,
        scroll_window_main_source, scroll_window_readme, tray_main_source, tray_readme,
        window_controls_main_source, window_controls_readme,
    },
    validation::{validate_display_label, validate_project_slug},
};

use super::InitError;

pub(super) fn initialize_template(
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
        TemplateKind::ContextMenu => (
            context_menu_main_source(display_label),
            context_menu_readme(&context),
            "Created Anodrel native context-menu project.",
        ),
        TemplateKind::Tray => (
            tray_main_source(display_label),
            tray_readme(&context),
            "Created Anodrel native tray project.",
        ),
        TemplateKind::Notification => (
            notification_main_source(display_label),
            notification_readme(&context),
            "Created Anodrel native notification project.",
        ),
        TemplateKind::FileWrite => (
            file_write_main_source(display_label),
            file_write_readme(&context),
            "Created Anodrel native file-write project.",
        ),
        TemplateKind::FileBinaryWrite => (
            file_binary_write_main_source(display_label),
            file_binary_write_readme(&context),
            "Created Anodrel native file-binary-write project.",
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
        TemplateKind::WindowControls => (
            window_controls_main_source(display_label),
            window_controls_readme(&context),
            "Created Anodrel native window-controls project.",
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
