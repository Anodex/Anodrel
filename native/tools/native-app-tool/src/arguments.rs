//! Exact command-line parsing for the native application generator.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateKind {
    Ui,
    Form,
    LiveStatus,
    Menu,
    ContextMenu,
    Tray,
    Notification,
    MultiWindow,
    ScrollWindow,
    WindowControls,
}

#[derive(Debug, Eq, PartialEq)]
pub struct InitCommand {
    pub template_kind: TemplateKind,
    pub destination: PathBuf,
    pub project_slug: String,
    pub display_label: String,
}

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<InitCommand, ()> {
    let mut arguments = arguments.into_iter();
    let template_kind = match arguments.next().as_deref() {
        Some("init") => TemplateKind::Ui,
        Some("init-form") => TemplateKind::Form,
        Some("init-live-status") => TemplateKind::LiveStatus,
        Some("init-menu") => TemplateKind::Menu,
        Some("init-context-menu") => TemplateKind::ContextMenu,
        Some("init-tray") => TemplateKind::Tray,
        Some("init-notification") => TemplateKind::Notification,
        Some("init-multi-window") => TemplateKind::MultiWindow,
        Some("init-scroll-window") => TemplateKind::ScrollWindow,
        Some("init-window-controls") => TemplateKind::WindowControls,
        _ => return Err(()),
    };
    let destination = arguments.next().ok_or(())?;
    let project_slug = arguments.next().ok_or(())?;
    let display_label = arguments.next().ok_or(())?;
    if arguments.next().is_some() {
        return Err(());
    }
    Ok(InitCommand {
        template_kind,
        destination: PathBuf::from(destination),
        project_slug,
        display_label,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{InitCommand, TemplateKind, parse};

    #[test]
    fn accepts_only_the_documented_init_form() {
        assert_eq!(
            parse(["init", "out/example", "example-app", "Example App"].map(String::from)),
            Ok(InitCommand {
                template_kind: TemplateKind::Ui,
                destination: PathBuf::from("out/example"),
                project_slug: "example-app".to_owned(),
                display_label: "Example App".to_owned(),
            })
        );
        assert_eq!(
            parse(["init-form", "out/form", "form-app", "Form App"].map(String::from)),
            Ok(InitCommand {
                template_kind: TemplateKind::Form,
                destination: PathBuf::from("out/form"),
                project_slug: "form-app".to_owned(),
                display_label: "Form App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-live-status",
                    "out/live-status",
                    "live-status-app",
                    "Live Status App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::LiveStatus,
                destination: PathBuf::from("out/live-status"),
                project_slug: "live-status-app".to_owned(),
                display_label: "Live Status App".to_owned(),
            })
        );
        assert_eq!(
            parse(["init-tray", "out/tray", "tray-app", "Tray App"].map(String::from)),
            Ok(InitCommand {
                template_kind: TemplateKind::Tray,
                destination: PathBuf::from("out/tray"),
                project_slug: "tray-app".to_owned(),
                display_label: "Tray App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-notification",
                    "out/notification",
                    "notification-app",
                    "Notification App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::Notification,
                destination: PathBuf::from("out/notification"),
                project_slug: "notification-app".to_owned(),
                display_label: "Notification App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-window-controls",
                    "out/window-controls",
                    "window-controls-app",
                    "Window Controls App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::WindowControls,
                destination: PathBuf::from("out/window-controls"),
                project_slug: "window-controls-app".to_owned(),
                display_label: "Window Controls App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-scroll-window",
                    "out/scroll-window",
                    "scroll-window-app",
                    "Scroll Window App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::ScrollWindow,
                destination: PathBuf::from("out/scroll-window"),
                project_slug: "scroll-window-app".to_owned(),
                display_label: "Scroll Window App".to_owned(),
            })
        );
        assert_eq!(
            parse(["init-menu", "out/menu", "menu-app", "Menu App"].map(String::from)),
            Ok(InitCommand {
                template_kind: TemplateKind::Menu,
                destination: PathBuf::from("out/menu"),
                project_slug: "menu-app".to_owned(),
                display_label: "Menu App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-context-menu",
                    "out/context-menu",
                    "context-menu-app",
                    "Context Menu App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::ContextMenu,
                destination: PathBuf::from("out/context-menu"),
                project_slug: "context-menu-app".to_owned(),
                display_label: "Context Menu App".to_owned(),
            })
        );
        assert_eq!(
            parse(
                [
                    "init-multi-window",
                    "out/multi-window",
                    "multi-window-app",
                    "Multi-Window App",
                ]
                .map(String::from)
            ),
            Ok(InitCommand {
                template_kind: TemplateKind::MultiWindow,
                destination: PathBuf::from("out/multi-window"),
                project_slug: "multi-window-app".to_owned(),
                display_label: "Multi-Window App".to_owned(),
            })
        );
        assert!(parse(["init", "out", "example"].map(String::from)).is_err());
        assert!(parse(["verify", "out"].map(String::from)).is_err());
        assert!(parse(["init", "out", "example", "Example", "extra"].map(String::from)).is_err());
    }
}
