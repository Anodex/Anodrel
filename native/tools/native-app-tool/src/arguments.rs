//! Exact command-line parsing for the native application generator.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemplateKind {
    Ui,
    Menu,
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
        Some("init-menu") => TemplateKind::Menu,
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
            parse(["init-menu", "out/menu", "menu-app", "Menu App"].map(String::from)),
            Ok(InitCommand {
                template_kind: TemplateKind::Menu,
                destination: PathBuf::from("out/menu"),
                project_slug: "menu-app".to_owned(),
                display_label: "Menu App".to_owned(),
            })
        );
        assert!(parse(["init", "out", "example"].map(String::from)).is_err());
        assert!(parse(["verify", "out"].map(String::from)).is_err());
        assert!(parse(["init", "out", "example", "Example", "extra"].map(String::from)).is_err());
    }
}
