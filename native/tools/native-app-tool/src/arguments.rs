//! Exact command-line parsing for the native application generator.

use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub struct InitCommand {
    pub destination: PathBuf,
    pub project_slug: String,
    pub display_label: String,
}

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<InitCommand, ()> {
    let mut arguments = arguments.into_iter();
    if arguments.next().as_deref() != Some("init") {
        return Err(());
    }
    let destination = arguments.next().ok_or(())?;
    let project_slug = arguments.next().ok_or(())?;
    let display_label = arguments.next().ok_or(())?;
    if arguments.next().is_some() {
        return Err(());
    }
    Ok(InitCommand {
        destination: PathBuf::from(destination),
        project_slug,
        display_label,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{InitCommand, parse};

    #[test]
    fn accepts_only_the_documented_init_form() {
        assert_eq!(
            parse(["init", "out/example", "example-app", "Example App"].map(String::from)),
            Ok(InitCommand {
                destination: PathBuf::from("out/example"),
                project_slug: "example-app".to_owned(),
                display_label: "Example App".to_owned(),
            })
        );
        assert!(parse(["init", "out", "example"].map(String::from)).is_err());
        assert!(parse(["verify", "out"].map(String::from)).is_err());
        assert!(parse(["init", "out", "example", "Example", "extra"].map(String::from)).is_err());
    }
}
